//! The `session` subcommand: the debug CLI that drives havoc-core over the
//! typed boundary. Command flow is built on the event stream, not the reply —
//! `Ack` means accepted and nothing more: `join` fires the request and returns,
//! and completion is observed with `wait buffer <name>`, which watches for the
//! `BufferCreated` event; `send` resolves channel names through the same
//! name→BufferId projection the TUI will keep. `wait` verbs exist so
//! live-run.sh never sleeps.
//!
//! **Every connection parameter comes from the config file** (prompt 10a): the
//! `--host`, `--port`, `--nick`, `--join`, `--tls-ca` and `--allow-plaintext`
//! flags are gone rather than kept as a fallback, because a fallback is what
//! makes live-run.sh keep exercising the flags and leaves config — the surface
//! every later stage builds on — the decorative one. `connect` is still an
//! explicit verb: nothing here autoconnects a configured network (stage 2's
//! embedded wiring owns that).

use std::collections::HashMap;
use std::path::Path;

use havoc_core::config;
use havoc_core::connection::io::Security;
use havoc_core::connection::{SaslConfig, SaslCredentials, SaslMechanism};
use havoc_core::core::Core;
use havoc_core::storage::Storage;
use havoc_ipc::{BufferId, ConnectionPhase, NetworkId, Request, RequestBody, RequestId, Seq};
use havoc_transport::{ClientTransport, InProcess};
use tokio::io::{AsyncBufReadExt, BufReader};

use crate::session_backlog::request_backlog;
use crate::session_wait::{Answered, Awaited, MsgCounts, finish, handle_incoming, wait_command};
use crate::wiring;

#[derive(clap::Args, Debug)]
pub struct SessionArgs {
    /// Which configured network this session drives. Optional when the config
    /// file names exactly one.
    #[arg(long)]
    pub network: Option<String>,
    /// SASL PLAIN account. The password comes from SUPERNAUT_SASL_PASSWORD —
    /// argv is world-readable in ps; env is the debug-grade bridge until
    /// prompt 10b's keyring, which deletes both.
    #[arg(long)]
    pub sasl: Option<String>,
    /// Echo raw IRC lines (`>>`/`<<`) to stderr — the transcript capture.
    /// Diagnostics, not seed data, so it stays a flag.
    #[arg(long)]
    pub trace_irc: bool,
    /// Where history lives. Stays a flag: a `data_dir` key in config would make
    /// "where is my history" answerable only by reading two files.
    #[arg(long)]
    pub data_dir: Option<std::path::PathBuf>,
}

pub async fn run(args: SessionArgs) -> Result<(), String> {
    let config_path = crate::default_config_path()?;
    let text = std::fs::read_to_string(&config_path).map_err(|e| {
        format!(
            "cannot read config {}: {e} — write the file, or point SUPERNAUT_CONFIG_DIR \
             at the directory holding it",
            config_path.display()
        )
    })?;
    // `tls_ca` resolves against the file's own directory, so a generated
    // fixture directory is portable.
    let base_dir = config_path.parent().unwrap_or(Path::new("."));
    let config = config::parse(&text, base_dir)?;
    let selected = resolve_network(&config, args.network.as_deref())?;

    // Core is spawned with **every** configured network, not just the selected
    // one: that is the `HashMap<NetworkId, _>` §6.9 asks for, it costs nothing
    // while nothing connects, and it is what lets the attach announcement
    // resolve buffers belonging to a configured network this process never
    // dialled — `send` then answers "buffer's network is not connected", which
    // is already the right sentence.
    let mut networks = config.into_networks();
    let network = *networks
        .iter()
        .find(|(_, settings)| settings.name == selected)
        .map(|(id, _)| id)
        .expect("the resolved name came from this same map");

    // One loud line at session start, per prompt 6 — now naming the network as
    // well as the host, because a process can hold several.
    let settings = &networks[&network];
    match &settings.security {
        Security::Plaintext => eprintln!(
            "PLAINTEXT session to network {} at {} — nothing on this connection is private",
            settings.name, settings.host
        ),
        Security::Tls {
            ca_file: Some(ca), ..
        } => eprintln!(
            "TLS with extra trust anchor {} for network {} (verification stays on)",
            ca.display(),
            settings.name
        ),
        Security::Tls { ca_file: None, .. } => {}
    }

    // **The one SASL site, and the one prompt 10b replaces with the keyring.**
    // Config lowers `sasl: None` always, because config has no credential
    // surface to lower from (NORTH-STAR §5.8) — so the credentials are injected
    // here, into the selected network only, after lowering.
    if let Some(account) = &args.sasl {
        let password = std::env::var("SUPERNAUT_SASL_PASSWORD")
            .map_err(|_| "--sasl requires SUPERNAUT_SASL_PASSWORD in the environment".to_owned())?;
        networks
            .get_mut(&network)
            .expect("selected network is in the map")
            .connection
            .sasl = Some(SaslConfig {
            mechanisms: vec![SaslMechanism::Plain],
            credentials: SaslCredentials {
                authcid: account.clone(),
                password,
            },
        });
    }

    let data_dir = match &args.data_dir {
        Some(dir) => dir.clone(),
        None => crate::default_data_dir()?,
    };
    std::fs::create_dir_all(&data_dir).map_err(|e| format!("data dir: {e}"))?;
    let (storage, _report) = Storage::open(&data_dir.join("history.db"), args.trace_irc)
        .map_err(|e| format!("storage: {e}"))?;

    let core = Core::spawn(storage.client(), networks, args.trace_irc);
    let transport = wiring::in_process(&core).await;

    // Storage's owning handle must outlive the session; keep it in scope.
    let result = drive(transport, network).await;
    drop(storage);
    result
}

/// `--network` is optional exactly when the file names one network; otherwise the
/// error names the candidates, because "ambiguous" without the list is a second
/// round trip through the user's own file.
fn resolve_network(config: &config::Config, requested: Option<&str>) -> Result<String, String> {
    let candidates: Vec<&str> = config.networks.keys().map(String::as_str).collect();
    let named = if candidates.is_empty() {
        "no networks".to_owned()
    } else {
        format!("[{}]", candidates.join(", "))
    };
    match requested {
        Some(name) if config.networks.contains_key(name) => Ok(name.to_owned()),
        Some(name) => Err(format!(
            "config: no [networks.{name}] table; the file names {named}"
        )),
        None => match candidates.as_slice() {
            [only] => Ok((*only).to_owned()),
            _ => Err(format!(
                "--network is required unless the config file names exactly one \
                 network; this one names {named}"
            )),
        },
    }
}

pub(crate) struct SessionState {
    pub(crate) transport: InProcess,
    /// The network this session drives, resolved from the config file — what
    /// `const NETWORK: NetworkId = NetworkId(1)` used to assert.
    pub(crate) network: NetworkId,
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

async fn drive(transport: InProcess, network: NetworkId) -> Result<(), String> {
    let mut state = SessionState {
        transport,
        network,
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
            // runtime drops underneath it. A malformed deadline is an error
            // rather than a silent 10 — a swallowed typo here would quietly
            // restore the race this drain exists to close.
            let secs = match parts.next() {
                None => 10,
                Some(arg) => match arg.parse() {
                    Ok(secs) => secs,
                    Err(_) => {
                        println!("error - quit takes a deadline in seconds, got {arg}");
                        return Ok(true);
                    }
                },
            };
            finish(state, secs).await?;
            Ok(false)
        }
        Some("connect") => {
            request(
                state,
                RequestBody::Connect {
                    network: state.network,
                },
            )
            .await?;
            Ok(true)
        }
        Some("join") => match parts.next() {
            Some(channel) => {
                request(
                    state,
                    RequestBody::Join {
                        network: state.network,
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
