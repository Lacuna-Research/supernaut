//! One tokio task per network: owns its [`Machine`] and its transport by
//! value, communicates by channels only (§5.5). The reconnect policy lives
//! here, inside `run()` — the Failed/Disconnected distinction never leaves
//! this task (`ActorReport` carries only the coarse phase), so the retry
//! decision must be made where `Machine::state()` is readable: fail-closed
//! `State::Failed` is never retried; every other loss backs off and retries.

use std::time::Duration;

use havoc_ipc::{ConnectionPhase, NetworkId};
use tokio::sync::mpsc;

use super::io::{AnyLineTransport, LineTransport, Security};
use super::{Config, Machine, State};

/// Commands the core sends its actor. Dropped while disconnected: autojoin
/// re-fires on the fresh machine, and a PRIVMSG delivered seconds after a
/// reconnect is a surprise, not a feature.
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
    pub security: Security,
    pub config: Config,
    pub reports: mpsc::Sender<(NetworkId, ActorReport)>,
    /// `>>`/`<<` raw-line trace to stderr — the capture the corpus harvests.
    pub trace: bool,
}

pub fn spawn(params: ActorSpawn) -> ActorHandle {
    let (commands, rx) = mpsc::channel(64);
    let task = tokio::spawn(run(params, rx));
    ActorHandle { commands, task }
}

/// Backoff: exponential from 1s doubling to a 60s cap, ±50% jitter derived
/// from the clock's subsecond nanos — a rand dependency for one jitter would
/// fail the allowlist's own justification bar.
fn backoff_delay(attempt: u32) -> Duration {
    let base = (1u64 << attempt.min(7).saturating_sub(1)).min(60); // 1,2,...,60
    // Micros, not the nanos tail: macOS's realtime clock is often
    // microsecond-granular, and `subsec_nanos() % 1_000` would be constantly
    // zero there — degenerate jitter on the one machine that runs this
    // (reviewer catch).
    let micros = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.subsec_micros())
        .unwrap_or(0);
    let factor = 0.5 + f64::from(micros % 1_000) / 1_000.0; // 0.5..1.5
    Duration::from_millis((base as f64 * 1_000.0 * factor) as u64)
}

/// How one connection attempt ended.
enum Attempt {
    /// Fail-closed (`State::Failed`): report and stop, never retry.
    Fatal(String),
    /// Transport lost; `registered` says whether this attempt got that far
    /// (which resets the backoff counter).
    Lost { detail: String, registered: bool },
    /// Core dropped the command channel: orderly shutdown.
    CommandsClosed,
}

async fn run(params: ActorSpawn, mut commands: mpsc::Receiver<ActorCommand>) {
    let ActorSpawn {
        network,
        host,
        port,
        security,
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

    let mut attempt: u32 = 0;
    loop {
        let connecting_detail = (attempt > 0).then(|| format!("retry {attempt}"));
        report_phase(ConnectionPhase::Connecting, connecting_detail).await;

        let spec = AttemptSpec {
            host: &host,
            port,
            security: &security,
            config: config.clone(),
            network,
            reports: &reports,
            trace,
        };
        match attempt_once(spec, &mut commands).await {
            Attempt::Fatal(reason) => {
                report_phase(ConnectionPhase::Disconnected, Some(reason)).await;
                return;
            }
            Attempt::CommandsClosed => return,
            Attempt::Lost { detail, registered } => {
                report_phase(ConnectionPhase::Disconnected, Some(detail)).await;
                attempt = if registered { 1 } else { attempt + 1 };
                let delay = backoff_delay(attempt);
                // Sleep, but keep draining (and dropping) commands so a
                // closed channel still exits the actor mid-backoff.
                let sleep = tokio::time::sleep(delay);
                tokio::pin!(sleep);
                loop {
                    tokio::select! {
                        () = &mut sleep => break,
                        command = commands.recv() => match command {
                            None => return,
                            Some(_) => { /* dropped while disconnected */ }
                        },
                    }
                }
            }
        }
    }
}

struct AttemptSpec<'a> {
    host: &'a str,
    port: u16,
    security: &'a Security,
    config: Config,
    network: NetworkId,
    reports: &'a mpsc::Sender<(NetworkId, ActorReport)>,
    trace: bool,
}

async fn attempt_once(
    spec: AttemptSpec<'_>,
    commands: &mut mpsc::Receiver<ActorCommand>,
) -> Attempt {
    let AttemptSpec {
        host,
        port,
        security,
        config,
        network,
        reports,
        trace,
    } = spec;
    let mut registered = false;

    let mut transport = match AnyLineTransport::connect(security, host, port).await {
        Ok(t) => t,
        Err(error) => {
            return Attempt::Lost {
                detail: format!("connect to {host}:{port} failed: {error}"),
                registered,
            };
        }
    };

    // A fresh machine per attempt — there is deliberately no reset() path.
    let (mut machine, opening) = Machine::start(config);
    for line in opening {
        if trace {
            eprintln!(">> {line}");
        }
        if transport.send_line(&line).await.is_err() {
            return Attempt::Lost {
                detail: "send failed".to_owned(),
                registered,
            };
        }
    }

    loop {
        tokio::select! {
            line = transport.next_line() => {
                let line = match line {
                    Ok(Some(line)) => line,
                    Ok(None) => {
                        machine.on_disconnect();
                        return Attempt::Lost {
                            detail: "server closed the connection".to_owned(),
                            registered,
                        };
                    }
                    Err(error) => {
                        machine.on_disconnect();
                        return Attempt::Lost {
                            detail: format!("read error: {error}"),
                            registered,
                        };
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
                        return Attempt::Lost {
                            detail: "send failed".to_owned(),
                            registered,
                        };
                    }
                }

                let after = machine.state().clone();
                if before != after {
                    if let State::Failed { reason } = &after {
                        return Attempt::Fatal(reason.clone());
                    }
                    if machine.phase() == ConnectionPhase::Registered {
                        registered = true;
                    }
                    let _ = reports
                        .send((network, ActorReport::Phase {
                            phase: machine.phase(),
                            detail: None,
                        }))
                        .await;
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
                    return Attempt::CommandsClosed;
                };
                let line = match command {
                    ActorCommand::Join(channel) => format!("JOIN {channel}"),
                    ActorCommand::Privmsg { target, text } => format!("PRIVMSG {target} :{text}"),
                };
                if trace {
                    eprintln!(">> {line}");
                }
                if transport.send_line(&line).await.is_err() {
                    return Attempt::Lost {
                        detail: "send failed".to_owned(),
                        registered,
                    };
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

    #[test]
    fn backoff_grows_and_caps() {
        for attempt in 1..12 {
            let d = super::backoff_delay(attempt);
            assert!(
                d >= std::time::Duration::from_millis(500),
                "floor at {attempt}"
            );
            assert!(d <= std::time::Duration::from_secs(90), "cap at {attempt}");
        }
    }
}
