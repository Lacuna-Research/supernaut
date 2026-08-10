//! The core task: request dispatch with per-session correlation, the two-lane
//! bus, and the actor map. Sessions attach and get typed channels; the binary
//! adapts them to havoc-transport's trait (no crate edge exists between core
//! and transport, deliberately — see the prompt-5 decision).
//!
//! `NetworkId`s on the wire are **caller-assigned** (config-level identity,
//! exactly what a config file gives an attached client too); the storage row
//! id stays core-private and is mapped here. That keeps embedded and attached
//! modes on identical footing (§4.3).

use std::collections::HashMap;

use havoc_ipc::{
    BufferId, BufferInfo, BufferKind, Event, NetworkId, Request, RequestBody, Response,
    ResponseBody,
};
use tokio::sync::{broadcast, mpsc};

use crate::bus::{Bus, ClientId, Directed};
use crate::connection::actor::{self, ActorCommand, ActorReport, ActorSpawn};
use crate::connection::{Config as ConnectionConfig, Networks};
use crate::storage::StorageClient;

/// Everything needed to reach one configured network. Flags today, config
/// file at prompt 10.
#[derive(Debug, Clone)]
pub struct NetworkSettings {
    pub name: String,
    pub host: String,
    pub port: u16,
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
        let (tx, rx) = mpsc::channel(64);
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
    /// Caller NetworkId → storage row id, filled at connect.
    network_rows: HashMap<NetworkId, havoc_ipc::NetworkId>,
    buffers: HashMap<BufferId, (NetworkId, String)>,
    networks: Networks,
    trace: bool,
    reports_tx: mpsc::Sender<(NetworkId, ActorReport)>,
}

async fn run(
    storage: StorageClient,
    settings: HashMap<NetworkId, NetworkSettings>,
    trace: bool,
    mut bus: Bus,
    mut requests: mpsc::Receiver<(ClientId, Request)>,
    mut attach: mpsc::Receiver<(ClientId, mpsc::Sender<Directed>)>,
) {
    let (reports_tx, mut reports) = mpsc::channel::<(NetworkId, ActorReport)>(256);
    let mut state = CoreState {
        storage,
        settings,
        network_rows: HashMap::new(),
        buffers: HashMap::new(),
        networks: Networks::default(),
        trace,
        reports_tx,
    };

    loop {
        tokio::select! {
            Some((client, lane)) = attach.recv() => {
                bus.register(client, lane);
            }
            Some((client, request)) = requests.recv() => {
                let response = handle_request(&mut state, request).await;
                bus.direct(client, Directed::Response(response)).await;
            }
            Some((network, report)) = reports.recv() => {
                handle_report(&mut state, &mut bus, network, report).await;
            }
            else => return,
        }
    }
}

async fn handle_request(state: &mut CoreState, request: Request) -> Response {
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
        RequestBody::FetchBacklog { .. } => error("FetchBacklog arrives in prompt 9".to_owned()),
        RequestBody::Search { .. } => error("Search arrives in prompt 8".to_owned()),
        RequestBody::SetReadMarker { .. } => error("SetReadMarker arrives in prompt 9".to_owned()),
    };
    Response { id, body }
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
        config: settings.connection.clone(),
        reports: state.reports_tx.clone(),
        trace: state.trace,
    });
    state.networks.insert(network, handle);
    ResponseBody::Ack
}

async fn handle_report(
    state: &mut CoreState,
    bus: &mut Bus,
    network: NetworkId,
    report: ActorReport,
) {
    match report {
        ActorReport::Phase { phase, detail } => {
            bus.broadcast(Event::ConnectionState {
                network,
                phase,
                detail,
            });
        }
        ActorReport::JoinedChannel(channel) => {
            let Some(row) = state.network_rows.get(&network).copied() else {
                return;
            };
            let storage = state.storage.clone();
            let name = channel.clone();
            let result = tokio::task::spawn_blocking(move || {
                storage.ensure_buffer(row, &name, BufferKind::Channel)
            })
            .await;
            let Ok(Ok(buffer)) = result else { return };
            state.buffers.insert(buffer, (network, channel.clone()));
            bus.broadcast(Event::BufferCreated {
                buffer: BufferInfo {
                    id: buffer,
                    network,
                    name: channel,
                    kind: BufferKind::Channel,
                    last_read_seq: None,
                },
            });
        }
    }
}

fn error(message: String) -> ResponseBody {
    ResponseBody::Error { message }
}
