//! The `session` subcommand: the debug CLI that drives havoc-core over the
//! typed boundary. Command flow is built on the event stream, not the reply —
//! `Ack` means accepted and nothing more: `join` fires the request and returns,
//! and completion is observed with `wait buffer <name>`, which watches for the
//! `BufferCreated` event; `send` resolves channel names through the same
//! name→BufferId projection the TUI will keep. `wait` verbs exist so
//! live-run.sh never sleeps.

use std::collections::HashMap;
use std::time::Duration;

use havoc_core::connection::io::Security;
use havoc_core::connection::{SaslConfig, SaslCredentials, SaslMechanism};
use havoc_core::core::{Core, NetworkSettings};
use havoc_core::storage::Storage;
use havoc_ipc::{BufferId, ConnectionPhase, Event, NetworkId, Request, RequestBody, RequestId};
use havoc_transport::{ClientTransport, InProcess, Incoming, TransportError};
use tokio::io::{AsyncBufReadExt, BufReader};

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
    let (storage, _report) =
        Storage::open(&data_dir.join("history.db")).map_err(|e| format!("storage: {e}"))?;

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

struct SessionState {
    transport: InProcess,
    next_request: u64,
    buffers: HashMap<String, BufferId>,
    phase: Option<ConnectionPhase>,
}

async fn drive(transport: InProcess) -> Result<(), String> {
    let mut state = SessionState {
        transport,
        next_request: 1,
        buffers: HashMap::new(),
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
                    None => return Ok(()),
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
    let mut parts = command.split_whitespace();
    match parts.next() {
        None => Ok(true),
        Some("quit") => Ok(false),
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
        Some("wait") => {
            let target = parts.next().unwrap_or("");
            let name = parts.next().map(str::to_owned);
            let secs: u64 = name
                .as_deref()
                .and_then(|n| n.parse().ok())
                .or_else(|| parts.next().and_then(|s| s.parse().ok()))
                .unwrap_or(10);
            wait(state, target, name, secs).await?;
            Ok(true)
        }
        Some(other) => {
            println!("error - unknown command {other}");
            Ok(true)
        }
    }
}

async fn request(state: &mut SessionState, body: RequestBody) -> Result<(), String> {
    let id = RequestId(state.next_request);
    state.next_request += 1;
    state
        .transport
        .send(Request { id, body })
        .await
        .map_err(|e| format!("send: {e}"))
}

/// `wait registered [secs]` / `wait buffer <name> [secs]`: consume (and still
/// print) events until the predicate holds or the deadline names itself.
async fn wait(
    state: &mut SessionState,
    target: &str,
    name: Option<String>,
    secs: u64,
) -> Result<(), String> {
    let satisfied = |state: &SessionState| match target {
        "registered" => state.phase == Some(ConnectionPhase::Registered),
        "buffer" => name
            .as_deref()
            .is_some_and(|n| state.buffers.contains_key(n)),
        _ => true,
    };
    if !matches!(target, "registered" | "buffer") {
        println!("error - wait knows 'registered' and 'buffer'");
        return Ok(());
    }

    let deadline = tokio::time::Instant::now() + Duration::from_secs(secs);
    while !satisfied(state) {
        let incoming = tokio::time::timeout_at(deadline, state.transport.recv()).await;
        match incoming {
            Ok(incoming) => handle_incoming(state, incoming)?,
            Err(_) => {
                return Err(format!(
                    "wait {target} {} timed out after {secs}s",
                    name.as_deref().unwrap_or("")
                ));
            }
        }
    }
    println!("waited {target} {}", name.as_deref().unwrap_or(""));
    Ok(())
}

fn handle_incoming(
    state: &mut SessionState,
    incoming: Result<Incoming, TransportError>,
) -> Result<(), String> {
    match incoming {
        Ok(Incoming::Response(response)) => {
            match response.body {
                havoc_ipc::ResponseBody::Ack => println!("ok {}", response.id.0),
                havoc_ipc::ResponseBody::Error { message } => {
                    println!("error {} {message}", response.id.0);
                }
                havoc_ipc::ResponseBody::Backlog { .. } => {
                    println!("ok {} (backlog)", response.id.0);
                }
            }
            Ok(())
        }
        Ok(Incoming::Event(event)) => {
            print_event(state, &event);
            Ok(())
        }
        // Lagged is loud but survivable; Closed ends the session.
        Err(TransportError::Lagged(n)) => {
            eprintln!("transport lagged: missed {n} events");
            Ok(())
        }
        Err(TransportError::Closed) => Err("transport closed".to_owned()),
    }
}

fn print_event(state: &mut SessionState, event: &Event) {
    match event {
        Event::ConnectionState {
            network,
            phase,
            detail,
        } => {
            let phase_str = match phase {
                ConnectionPhase::Connecting => "connecting",
                ConnectionPhase::Registered => "registered",
                ConnectionPhase::Disconnected => "disconnected",
            };
            state.phase = Some(*phase);
            match detail {
                Some(detail) => println!(
                    "event connection-state network={} phase={phase_str} detail={detail}",
                    network.0
                ),
                None => println!(
                    "event connection-state network={} phase={phase_str}",
                    network.0
                ),
            }
        }
        Event::BufferCreated { buffer } => {
            state.buffers.insert(buffer.name.clone(), buffer.id);
            println!(
                "event buffer-created buffer={} network={} name={}",
                buffer.id.0, buffer.network.0, buffer.name
            );
        }
        Event::MessageAdded { message } => {
            println!(
                "event message-added buffer={} seq={}",
                message.buffer.0, message.seq.0
            );
        }
        Event::SearchResults { request, hits } => {
            println!(
                "event search-results request={} hits={}",
                request.0,
                hits.len()
            );
        }
        Event::ReadMarkerChanged { buffer, seq } => {
            println!("event read-marker buffer={} seq={}", buffer.0, seq.0);
        }
    }
}

fn is_loopback(host: &str) -> bool {
    host == "localhost" || host == "127.0.0.1" || host == "::1"
}
