//! One tokio task per network: owns its [`Machine`] and its transport by
//! value, communicates by channels only (§5.5). Emits phase changes by
//! diffing `state()` around each line — keyed on `state()`, not `phase()`,
//! because prompt 6's backoff must distinguish `Failed` from `Disconnected`
//! and `phase()` folds them.

use havoc_ipc::{ConnectionPhase, NetworkId};
use tokio::sync::mpsc;

use super::io::{LineTransport, TcpLineTransport};
use super::{Config, Machine, State};

/// Commands the core sends its actor.
#[derive(Debug)]
pub enum ActorCommand {
    Join(String),
    Privmsg { target: String, text: String },
}

/// What the actor reports back to the core task.
#[derive(Debug)]
pub enum ActorReport {
    Phase {
        phase: ConnectionPhase,
        detail: Option<String>,
    },
    /// The server confirmed *our* join of this channel.
    JoinedChannel(String),
}

/// Handle held in the `Networks` map: the command lane plus the task itself.
#[derive(Debug)]
pub struct ActorHandle {
    pub commands: mpsc::Sender<ActorCommand>,
    pub task: tokio::task::JoinHandle<()>,
}

pub struct ActorSpawn {
    pub network: NetworkId,
    pub host: String,
    pub port: u16,
    pub config: Config,
    pub reports: mpsc::Sender<(NetworkId, ActorReport)>,
    /// `>>`/`<<` raw-line trace to stderr — the capture prompt 6 harvests.
    pub trace: bool,
}

pub fn spawn(params: ActorSpawn) -> ActorHandle {
    let (commands, rx) = mpsc::channel(64);
    let task = tokio::spawn(run(params, rx));
    ActorHandle { commands, task }
}

async fn run(params: ActorSpawn, mut commands: mpsc::Receiver<ActorCommand>) {
    let ActorSpawn {
        network,
        host,
        port,
        config,
        reports,
        trace,
    } = params;

    let report_phase = |phase, detail: Option<String>| {
        let reports = reports.clone();
        async move {
            let _ = reports
                .send((network, ActorReport::Phase { phase, detail }))
                .await;
        }
    };

    report_phase(ConnectionPhase::Connecting, None).await;

    let mut transport = match TcpLineTransport::connect(&host, port).await {
        Ok(t) => t,
        Err(error) => {
            report_phase(
                ConnectionPhase::Disconnected,
                Some(format!("connect to {host}:{port} failed: {error}")),
            )
            .await;
            return;
        }
    };

    let (mut machine, opening) = Machine::start(config);
    for line in opening {
        if trace {
            eprintln!(">> {line}");
        }
        if transport.send_line(&line).await.is_err() {
            report_phase(ConnectionPhase::Disconnected, Some("send failed".into())).await;
            return;
        }
    }

    loop {
        tokio::select! {
            line = transport.next_line() => {
                let line = match line {
                    Ok(Some(line)) => line,
                    Ok(None) => {
                        machine.on_disconnect();
                        report_phase(ConnectionPhase::Disconnected, Some("server closed the connection".into())).await;
                        return;
                    }
                    Err(error) => {
                        machine.on_disconnect();
                        report_phase(ConnectionPhase::Disconnected, Some(format!("read error: {error}"))).await;
                        return;
                    }
                };
                if trace {
                    eprintln!("<< {line}");
                }

                let before = machine.state().clone();
                let replies = machine.handle_line(&line);
                for reply in replies {
                    if trace {
                        eprintln!(">> {reply}");
                    }
                    if transport.send_line(&reply).await.is_err() {
                        report_phase(ConnectionPhase::Disconnected, Some("send failed".into())).await;
                        return;
                    }
                }

                let after = machine.state().clone();
                if before != after {
                    let detail = match &after {
                        State::Failed { reason } => Some(reason.clone()),
                        _ => None,
                    };
                    report_phase(machine.phase(), detail).await;
                    if matches!(after, State::Failed { .. }) {
                        // Fail-closed states end the attempt; retry policy is
                        // prompt 6's, and it must read state(), never phase().
                        return;
                    }
                }

                // The machine parses internally and discards non-protocol
                // messages (prompt-7 note owns the parse-once seam). The one
                // thing the wiring needs today — our own confirmed JOIN — is
                // detected here with a second, local parse.
                if let Some(channel) = our_join(&line, machine.nick()) {
                    let _ = reports.send((network, ActorReport::JoinedChannel(channel))).await;
                }
            }
            command = commands.recv() => {
                let Some(command) = command else {
                    // Core dropped the handle: orderly shutdown.
                    return;
                };
                let line = match command {
                    ActorCommand::Join(channel) => format!("JOIN {channel}"),
                    ActorCommand::Privmsg { target, text } => format!("PRIVMSG {target} :{text}"),
                };
                if trace {
                    eprintln!(">> {line}");
                }
                if transport.send_line(&line).await.is_err() {
                    report_phase(ConnectionPhase::Disconnected, Some("send failed".into())).await;
                    return;
                }
            }
        }
    }
}

/// Does this line confirm our own JOIN? (`:nick!user@host JOIN #chan`)
fn our_join(line: &str, our_nick: &str) -> Option<String> {
    let message: irc_proto::Message = line.parse().ok()?;
    let irc_proto::Command::JOIN(channels, _, _) = &message.command else {
        return None;
    };
    let nick = match &message.prefix {
        Some(irc_proto::Prefix::Nickname(nick, _, _)) => nick,
        _ => return None,
    };
    (nick == our_nick).then(|| channels.clone())
}

#[cfg(test)]
mod tests {
    #[test]
    fn our_join_matches_only_our_nick() {
        assert_eq!(
            super::our_join(":hvc!u@h JOIN #supernaut", "hvc").as_deref(),
            Some("#supernaut")
        );
        assert_eq!(super::our_join(":bob!u@h JOIN #supernaut", "hvc"), None);
        assert_eq!(super::our_join(":irc.example 001 hvc :hi", "hvc"), None);
    }
}
