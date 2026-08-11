//! The core task: request dispatch with per-session correlation, the two-lane
//! bus, and the actor map. Sessions attach and get typed channels; the binary
//! adapts them to havoc-transport's trait (no crate edge exists between core
//! and transport, deliberately — see the prompt-5 decision).
//!
//! `NetworkId`s on the wire are **caller-assigned** (config-level identity,
//! exactly what a config file gives an attached client too); the storage row
//! id stays core-private and is mapped here. That keeps embedded and attached
//! modes on identical footing (§4.3). Since prompt 10a the assigner is the
//! config *loader* — networks sorted by name, `NetworkId(1..N)` — not a human
//! typing an `id` key; nothing persists a wire id, so renumbering across runs
//! is unobservable.

mod reads;

use std::collections::HashMap;

use havoc_ipc::{
    BufferId, BufferInfo, Event, NetworkId, Request, RequestBody, Response, ResponseBody,
};
use tokio::sync::{broadcast, mpsc};

use crate::bus::{Bus, ClientId, DIRECTED_LANE_CAPACITY, Directed};
use crate::connection::actor::{self, ActorCommand, ActorReport, ActorSpawn};
use crate::connection::io::Security;
use crate::connection::{Config as ConnectionConfig, Networks};
use crate::storage::{IngestOutcome, NetworkRow, ReadOutcome, SearchOutcome, StorageClient};
use reads::{handle_read_outcome, handle_search_outcome, wire};

/// Everything needed to reach one configured network — lowered from the TOML
/// file by [`crate::config::Config::into_networks`] since prompt 10a. `name` is
/// the config table key, which is also the string storage keys the `network`
/// table on: one identity, not two that must agree.
#[derive(Debug, Clone)]
pub struct NetworkSettings {
    pub name: String,
    pub host: String,
    pub port: u16,
    pub security: Security,
    pub connection: ConnectionConfig,
}

/// One attached client's channels. The binary wraps this in the transport
/// trait; stage 4's accept loop will produce the same shape from a socket.
pub struct Session {
    pub id: ClientId,
    pub requests: mpsc::Sender<(ClientId, Request)>,
    pub directed: mpsc::Receiver<Directed>,
    pub broadcast: broadcast::Receiver<Event>,
}

pub struct CoreHandle {
    requests: mpsc::Sender<(ClientId, Request)>,
    attach: mpsc::Sender<(ClientId, mpsc::Sender<Directed>)>,
    broadcast_handle: broadcast::Sender<Event>,
    next_client: std::sync::atomic::AtomicU64,
}

impl CoreHandle {
    pub async fn attach(&self) -> Session {
        let id = ClientId(
            self.next_client
                .fetch_add(1, std::sync::atomic::Ordering::Relaxed),
        );
        let (tx, rx) = mpsc::channel(DIRECTED_LANE_CAPACITY);
        let broadcast = self.broadcast_handle.subscribe();
        self.attach.send((id, tx)).await.expect("core task alive");
        Session {
            id,
            requests: self.requests.clone(),
            directed: rx,
            broadcast,
        }
    }
}

pub struct Core;

impl Core {
    pub fn spawn(
        storage: StorageClient,
        networks: HashMap<NetworkId, NetworkSettings>,
        trace: bool,
    ) -> CoreHandle {
        let (req_tx, req_rx) = mpsc::channel::<(ClientId, Request)>(256);
        let (attach_tx, attach_rx) = mpsc::channel(16);
        let bus = Bus::new();
        let broadcast_sender = bus.broadcast_sender();

        tokio::spawn(run(storage, networks, trace, bus, req_rx, attach_rx));

        CoreHandle {
            requests: req_tx,
            attach: attach_tx,
            broadcast_handle: broadcast_sender,
            next_client: std::sync::atomic::AtomicU64::new(1),
        }
    }
}

struct CoreState {
    storage: StorageClient,
    settings: HashMap<NetworkId, NetworkSettings>,
    /// Caller NetworkId → storage row, filled at connect. Different types by
    /// design: the id spaces can no longer be swapped silently.
    network_rows: HashMap<NetworkId, NetworkRow>,
    buffers: HashMap<BufferId, (NetworkId, String)>,
    networks: Networks,
    trace: bool,
    reports_tx: mpsc::UnboundedSender<(NetworkId, ActorReport)>,
    outcome_tx: mpsc::Sender<IngestOutcome>,
    search_tx: mpsc::Sender<SearchOutcome>,
    reads_tx: mpsc::Sender<ReadOutcome>,
}

async fn run(
    storage: StorageClient,
    settings: HashMap<NetworkId, NetworkSettings>,
    trace: bool,
    mut bus: Bus,
    mut requests: mpsc::Receiver<(ClientId, Request)>,
    mut attach: mpsc::Receiver<(ClientId, mpsc::Sender<Directed>)>,
) {
    let (reports_tx, mut reports) = mpsc::unbounded_channel::<(NetworkId, ActorReport)>();
    let (outcome_tx, mut outcomes) = mpsc::channel::<IngestOutcome>(1024);
    let (search_tx, mut searches) = mpsc::channel::<SearchOutcome>(64);
    let (reads_tx, mut reads) = mpsc::channel::<ReadOutcome>(64);
    let mut state = CoreState {
        storage,
        settings,
        network_rows: HashMap::new(),
        buffers: HashMap::new(),
        networks: Networks::default(),
        trace,
        reports_tx,
        outcome_tx,
        search_tx,
        reads_tx,
    };

    loop {
        tokio::select! {
            Some((client, lane)) = attach.recv() => {
                bus.register(client, lane);
                // §4.5's attach contract: a fresh client is *told* what buffers
                // exist. Not a request — RequestBody grows nothing, so §4.7's
                // fence is not approached, let alone crossed.
                if let Err(e) = state.storage.list_buffers(client, state.reads_tx.clone()) {
                    eprintln!("attach: buffer announcement unavailable: {e}");
                }
            }
            Some((client, request)) = requests.recv() => {
                if let Some(response) = handle_request(&mut state, client, request).await {
                    bus.direct(client, Directed::Response(response));
                }
            }
            Some((network, report)) = reports.recv() => {
                handle_report(&mut state, &mut bus, network, report);
            }
            Some(outcome) = outcomes.recv() => {
                handle_outcome(&mut state, &bus, outcome);
            }
            Some(search) = searches.recv() => {
                handle_search_outcome(&mut bus, search);
            }
            Some(read) = reads.recv() => {
                handle_read_outcome(&mut state, &mut bus, read);
            }
            else => return,
        }
    }
}

/// `None` means the response is deferred (search: the exactly-one-Response
/// contract promises one response, not an instant one).
async fn handle_request(
    state: &mut CoreState,
    client: crate::bus::ClientId,
    request: Request,
) -> Option<Response> {
    let id = request.id;
    let body = match request.body {
        RequestBody::Connect { network } => connect(state, network).await,
        RequestBody::Join { network, channel } => match state.networks.get(network) {
            Some(handle) => {
                let _ = handle.commands.send(ActorCommand::Join(channel)).await;
                ResponseBody::Ack
            }
            None => error(format!("network {} is not connected", network.0)),
        },
        RequestBody::SendText { buffer, text } => match state.buffers.get(&buffer) {
            Some((network, target)) => match state.networks.get(*network) {
                Some(handle) => {
                    let command = ActorCommand::Privmsg {
                        target: target.clone(),
                        text,
                    };
                    let _ = handle.commands.send(command).await;
                    ResponseBody::Ack
                }
                None => error("buffer's network is not connected".to_owned()),
            },
            None => error(format!("unknown buffer {}", buffer.0)),
        },
        RequestBody::FetchBacklog {
            buffer,
            anchor,
            limit,
        } => {
            // A failed enqueue must not swallow the one promised Response —
            // the same reviewer catch the Search arm carries.
            match state
                .storage
                .backlog(buffer, anchor, limit, client, id, state.reads_tx.clone())
            {
                Ok(()) => return None,
                Err(e) => error(format!("storage unavailable: {e}")),
            }
        }
        RequestBody::Search { query } => match crate::search::parse(&query) {
            Err(message) => error(message),
            Ok(spec) => {
                // A failed enqueue must not swallow the one promised
                // Response (reviewer catch).
                match state
                    .storage
                    .search(spec, client, id, state.search_tx.clone())
                {
                    Ok(()) => return None,
                    Err(e) => error(format!("storage unavailable: {e}")),
                }
            }
        },
        // A write with a deferred response, exactly as Search and
        // FetchBacklog are: the job goes behind the flush barrier and the
        // outcome carries both the Ack and the broadcast event. A failed
        // enqueue must not swallow the one promised Response.
        RequestBody::SetReadMarker { buffer, seq } => {
            match state
                .storage
                .set_read_marker(buffer, seq, client, id, state.reads_tx.clone())
            {
                Ok(()) => return None,
                Err(e) => error(format!("storage unavailable: {e}")),
            }
        }
    };
    Some(Response { id, body })
}

async fn connect(state: &mut CoreState, network: NetworkId) -> ResponseBody {
    let Some(settings) = state.settings.get(&network).cloned() else {
        return error(format!("unknown network {}", network.0));
    };
    if state.networks.get(network).is_some() {
        return error(format!("network {} is already connected", network.0));
    }

    // Storage blocks on its job channel; never call it inline from the
    // executor (prompt-3 carry-forward). Two call sites, deliberately no
    // facade — prompt 7 decides that with the flood test in front of it.
    let storage = state.storage.clone();
    let name = settings.name.clone();
    let row = tokio::task::spawn_blocking(move || storage.ensure_network(&name)).await;
    let row = match row {
        Ok(Ok(row)) => row,
        Ok(Err(e)) => return error(format!("storage: {e}")),
        Err(e) => return error(format!("storage task: {e}")),
    };
    state.network_rows.insert(network, row);

    let handle = actor::spawn(ActorSpawn {
        network,
        host: settings.host.clone(),
        port: settings.port,
        security: settings.security.clone(),
        config: settings.connection.clone(),
        reports: state.reports_tx.clone(),
        trace: state.trace,
    });
    state.networks.insert(network, handle);
    ResponseBody::Ack
}

fn handle_report(state: &mut CoreState, bus: &mut Bus, network: NetworkId, report: ActorReport) {
    match report {
        ActorReport::Phase { phase, detail } => {
            bus.broadcast(Event::ConnectionState {
                network,
                phase,
                detail,
            });
        }
        // The no-await ingest lane: a plain non-blocking send into the storage
        // job queue. The flood cannot serialize through this select loop.
        ActorReport::Message(item) => {
            let Some(row) = state.network_rows.get(&network).copied() else {
                // Structurally unreachable (connect fills the map before the
                // actor exists) — but history must never vanish silently.
                eprintln!("ingest dropped: unknown network {}", network.0);
                return;
            };
            let _ = state
                .storage
                .ingest(network, row, item, state.outcome_tx.clone());
        }
    }
}

/// Post-commit: emit BufferCreated only on first touch per core instance —
/// across a reconnect the replayed autojoin re-ensures the same row and emits
/// nothing — then MessageAdded per inserted row, in order.
fn handle_outcome(state: &mut CoreState, bus: &Bus, outcome: IngestOutcome) {
    let known = state.buffers.contains_key(&outcome.buffer);
    state.buffers.insert(
        outcome.buffer,
        (outcome.network, outcome.buffer_name.clone()),
    );
    if outcome.buffer_created && !known {
        bus.broadcast(Event::BufferCreated {
            buffer: BufferInfo {
                id: outcome.buffer,
                network: outcome.network,
                name: outcome.buffer_name,
                kind: outcome.buffer_kind,
                last_read_seq: None,
            },
        });
    }
    if let Some(message) = outcome.message {
        bus.broadcast(Event::MessageAdded {
            message: wire(outcome.buffer, message),
        });
    }
}

fn error(message: String) -> ResponseBody {
    ResponseBody::Error { message }
}
