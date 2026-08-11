//! The `session` subcommand: the debug CLI that drives havoc-core over the
//! typed boundary. Command flow is built on the event stream, not the reply —
//! `Ack` means accepted and nothing more: `join` fires the request and returns,
//! and completion is observed with `wait buffer <name>`, which watches for the
//! `BufferCreated` event; `send` resolves channel names through the same
//! name→BufferId projection the TUI will keep. `wait` verbs exist so
//! live-run.sh never sleeps.

use std::collections::HashMap;

use havoc_core::connection::io::Security;
use havoc_core::connection::{SaslConfig, SaslCredentials, SaslMechanism};
use havoc_core::core::{Core, NetworkSettings};
use havoc_core::storage::Storage;
use havoc_ipc::{BufferId, ConnectionPhase, NetworkId, Request, RequestBody, RequestId, Seq};
use havoc_transport::{ClientTransport, InProcess};
use tokio::io::{AsyncBufReadExt, BufReader};

use crate::session_backlog::request_backlog;
use crate::session_wait::{Answered, Awaited, MsgCounts, finish, handle_incoming, wait_command};
use crate::wiring;

#[derive(clap::Args, Debug)]
pub struct SessionArgs {
    #[arg(long)]
    pub host: String,
    #[arg(long)]
    pub port: u16,
    #[arg(long)]
    pub nick: String,
    /// Channels to autojoin at registration.
    #[arg(long)]
    pub join: Vec<String>,
    /// Plaintext is the loud opt-in (NORTH-STAR §2.3) and loopback-only;
    /// without it, the session dials TLS and verifies it.
    #[arg(long)]
    pub allow_plaintext: bool,
    /// Extra PEM trust anchor appended to webpki-roots (for a local ergo's
    /// generated cert). Verification stays ON — there is no skip-verify.
    #[arg(long)]
    pub tls_ca: Option<std::path::PathBuf>,
    /// SASL PLAIN account. The password comes from SUPERNAUT_SASL_PASSWORD —
    /// argv is world-readable in ps; env is the debug-grade bridge until
    /// prompt 10's keyring.
    #[arg(long)]
    pub sasl: Option<String>,
    /// Echo raw IRC lines (`>>`/`<<`) to stderr — the transcript capture.
    #[arg(long)]
    pub trace_irc: bool,
    #[arg(long)]
    pub data_dir: Option<std::path::PathBuf>,
}

const NETWORK: NetworkId = NetworkId(1);

pub async fn run(args: SessionArgs) -> Result<(), String> {
    let security = if args.allow_plaintext {
        if !is_loopback(&args.host) {
            return Err(format!(
                "--allow-plaintext permits loopback only; {} is not 127.0.0.1/::1/localhost",
                args.host
            ));
        }
        eprintln!(
            "PLAINTEXT session to {} — nothing on this connection is private",
            args.host
        );
        Security::Plaintext
    } else {
        if let Some(ca) = &args.tls_ca {
            eprintln!(
                "TLS with extra trust anchor {} (verification stays on)",
                ca.display()
            );
        }
        Security::Tls {
            server_name: args.host.clone(),
            ca_file: args.tls_ca.clone(),
        }
    };

    let sasl = match &args.sasl {
        None => None,
        Some(account) => {
            let password = std::env::var("SUPERNAUT_SASL_PASSWORD").map_err(|_| {
                "--sasl requires SUPERNAUT_SASL_PASSWORD in the environment".to_owned()
            })?;
            Some(SaslConfig {
                mechanisms: vec![SaslMechanism::Plain],
                credentials: SaslCredentials {
                    authcid: account.clone(),
                    password,
                },
            })
        }
    };

    let data_dir = match &args.data_dir {
        Some(dir) => dir.clone(),
        None => crate::default_data_dir()?,
    };
    std::fs::create_dir_all(&data_dir).map_err(|e| format!("data dir: {e}"))?;
    let (storage, _report) = Storage::open(&data_dir.join("history.db"), args.trace_irc)
        .map_err(|e| format!("storage: {e}"))?;

    let settings = NetworkSettings {
        name: format!("debug-{}", args.host),
        host: args.host.clone(),
        port: args.port,
        security,
        connection: havoc_core::connection::Config {
            nick: args.nick.clone(),
            username: args.nick.clone(),
            realname: "supernaut debug session".to_owned(),
            sasl,
            autojoin: args.join.clone(),
        },
    };
    let core = Core::spawn(
        storage.client(),
        HashMap::from([(NETWORK, settings)]),
        args.trace_irc,
    );
    let transport = wiring::in_process(&core).await;

    // Storage's owning handle must outlive the session; keep it in scope.
    let result = drive(transport).await;
    drop(storage);
    result
}

pub(crate) struct SessionState {
    pub(crate) transport: InProcess,
    pub(crate) next_request: u64,
    pub(crate) buffers: HashMap<String, BufferId>,
    /// Per-buffer message tallies, kind-aware: see [`MsgCounts`].
    pub(crate) msg_counts: HashMap<BufferId, MsgCounts>,
    /// The newest search hit per buffer — what `backlog <b> around-hit` uses.
    pub(crate) last_hits: HashMap<BufferId, Seq>,
    /// Every request that has not been answered yet, classified at send time.
    /// One structure with one insert site (`request`) and one remove site
    /// (`handle_incoming`), so a new verb cannot forget to register — and so
    /// `quit` knows what it is still owed.
    pub(crate) outstanding: HashMap<RequestId, Awaited>,
    pub(crate) answered: Answered,
    pub(crate) phase: Option<ConnectionPhase>,
}

async fn drive(transport: InProcess) -> Result<(), String> {
    let mut state = SessionState {
        transport,
        next_request: 1,
        buffers: HashMap::new(),
        msg_counts: HashMap::new(),
        last_hits: HashMap::new(),
        outstanding: HashMap::new(),
        answered: Answered::default(),
        phase: None,
    };

    let stdin = BufReader::new(tokio::io::stdin());
    let mut commands = stdin.lines();

    loop {
        tokio::select! {
            line = commands.next_line() => {
                match line.map_err(|e| format!("stdin: {e}"))? {
                    Some(command) => {
                        if !dispatch(&mut state, command.trim()).await? {
                            return Ok(());
                        }
                    }
                    // stdin EOF is a quit: drain, or the runtime drop
                    // discards everything still in flight.
                    None => return finish(&mut state, 10).await,
                }
            }
            incoming = state.transport.recv() => {
                handle_incoming(&mut state, incoming)?;
            }
        }
    }
}

/// Returns Ok(false) on `quit`.
async fn dispatch(state: &mut SessionState, command: &str) -> Result<bool, String> {
    // `search` takes the raw remainder — whitespace-splitting would destroy
    // the quoting the core-side grammar depends on.
    if let Some(rest) = command.strip_prefix("search ") {
        request(
            state,
            RequestBody::Search {
                query: rest.trim().to_owned(),
            },
        )
        .await?;
        return Ok(true);
    }
    let mut parts = command.split_whitespace();
    match parts.next() {
        None => Ok(true),
        Some("quit") => {
            // `quit [secs]`: wait for what the engine still owes before the
            // runtime drops underneath it.
            finish(
                state,
                parts.next().and_then(|s| s.parse().ok()).unwrap_or(10),
            )
            .await?;
            Ok(false)
        }
        Some("connect") => {
            request(state, RequestBody::Connect { network: NETWORK }).await?;
            Ok(true)
        }
        Some("join") => match parts.next() {
            Some(channel) => {
                request(
                    state,
                    RequestBody::Join {
                        network: NETWORK,
                        channel: channel.to_owned(),
                    },
                )
                .await?;
                Ok(true)
            }
            None => {
                println!("error - join requires a channel");
                Ok(true)
            }
        },
        Some("send") => {
            let Some(channel) = parts.next() else {
                println!("error - send requires a channel and text");
                return Ok(true);
            };
            let text: String = parts.collect::<Vec<_>>().join(" ");
            let Some(&buffer) = state.buffers.get(channel) else {
                println!("error - no buffer for {channel} (join it first)");
                return Ok(true);
            };
            request(state, RequestBody::SendText { buffer, text }).await?;
            Ok(true)
        }
        Some("mark-read") => {
            let Some(name) = parts.next() else {
                println!("error - mark-read requires a buffer and a seq");
                return Ok(true);
            };
            let Some(seq) = parts.next().and_then(|s| s.parse().ok()) else {
                println!("error - mark-read requires a seq");
                return Ok(true);
            };
            // The same name->BufferId projection `send` and `backlog` use, which
            // includes buffers this process only learned of from the attach
            // announcement.
            let Some(&buffer) = state.buffers.get(name) else {
                println!("error - no buffer for {name} (join it, or attach over its history)");
                return Ok(true);
            };
            request(
                state,
                RequestBody::SetReadMarker {
                    buffer,
                    seq: Seq(seq),
                },
            )
            .await?;
            Ok(true)
        }
        Some("backlog") => {
            let rest: Vec<&str> = parts.collect();
            request_backlog(state, &rest).await?;
            Ok(true)
        }
        Some("wait") => {
            let rest: Vec<&str> = parts.collect();
            wait_command(state, &rest).await?;
            Ok(true)
        }
        Some(other) => {
            println!("error - unknown command {other}");
            Ok(true)
        }
    }
}

pub(crate) async fn request(
    state: &mut SessionState,
    body: RequestBody,
) -> Result<RequestId, String> {
    let id = RequestId(state.next_request);
    state.next_request += 1;
    // Classified from the body itself: the one insert site, so a verb added
    // later cannot forget to register and leave `quit` unaware of it.
    let awaited = Awaited::from(&body);
    state
        .transport
        .send(Request { id, body })
        .await
        .map_err(|e| format!("send: {e}"))?;
    state.outstanding.insert(id, awaited);
    Ok(id)
}

fn is_loopback(host: &str) -> bool {
    host == "localhost" || host == "127.0.0.1" || host == "::1"
}
