//! Delivery of read answers: search hits and backlog windows out to the session
//! that asked, and the attach-time buffer announcement. Split from core.rs for
//! the size ratchet; the grouping is the read path's whole delivery story, and
//! the two lane disciplines it uses sit next to each other on purpose —
//! [`Bus::direct`] for search's ordered Response-then-Event pair, the
//! non-blocking [`Bus::try_direct`] for a window.

use std::collections::HashMap;

use havoc_ipc::{BufferId, BufferInfo, Event, NetworkId, Response, ResponseBody};

use super::CoreState;
use crate::bus::{Bus, ClientId, Directed};
use crate::storage::{ReadOutcome, SearchOutcome};

/// Response first (Ack, or the SQLite error for a malformed MATCH — user
/// input never hangs and is never swallowed), then the correlated hits on
/// the directed lane only. Errors get no event.
pub(super) async fn handle_search_outcome(bus: &mut Bus, outcome: SearchOutcome) {
    let SearchOutcome {
        client,
        request,
        result,
    } = outcome;
    match result {
        Err(message) => {
            bus.direct(
                client,
                Directed::Response(Response {
                    id: request,
                    body: ResponseBody::Error { message },
                }),
            )
            .await;
        }
        Ok(rows) => {
            bus.direct(
                client,
                Directed::Response(Response {
                    id: request,
                    body: ResponseBody::Ack,
                }),
            )
            .await;
            let hits = rows
                .into_iter()
                .map(|(buffer, m)| wire(buffer, m))
                .collect();
            bus.direct(
                client,
                Directed::Event(Event::SearchResults { request, hits }),
            )
            .await;
        }
    }
}

/// Read answers go out non-blocking ([`Bus::try_direct`]): a window is a big
/// payload and a read is re-askable, so a reader that has stopped draining
/// loses its lane rather than stalling the select loop and the storage thread
/// behind it.
pub(super) fn handle_read_outcome(state: &mut CoreState, bus: &mut Bus, outcome: ReadOutcome) {
    match outcome {
        ReadOutcome::Backlog {
            client,
            request,
            result,
        } => {
            let body = match result {
                Ok(rows) => ResponseBody::Backlog {
                    messages: rows
                        .into_iter()
                        .map(|(buffer, m)| wire(buffer, m))
                        .collect(),
                },
                Err(message) => ResponseBody::Error { message },
            };
            bus.try_direct(client, Directed::Response(Response { id: request, body }));
        }
        ReadOutcome::Buffers { client, result } => match result {
            Err(message) => eprintln!("attach: buffer announcement failed: {message}"),
            Ok(rows) => announce(state, bus, client, rows),
        },
    }
}

/// Announce the buffers this client cannot have seen created, and seed
/// `state.buffers` from the same list — which is what makes `SendText` to a
/// buffer from a previous run answer "buffer's network is not connected"
/// instead of "unknown buffer".
///
/// Delivery is a short-lived task holding a clone of the session's lane: a
/// just-attached client may not be reading yet and the list can exceed the
/// lane's 64 slots, so neither the core loop nor the storage thread may wait on
/// it. Ordering against broadcast traffic is deliberately not guaranteed; the
/// contract is that announcements are **idempotent** — a duplicate is legal, a
/// missing one is not — which is also what makes the race between this snapshot
/// and a concurrent `BufferCreated` a non-event.
fn announce(
    state: &mut CoreState,
    bus: &Bus,
    client: ClientId,
    rows: Vec<crate::storage::BufferRow>,
) {
    let by_name: HashMap<&str, NetworkId> = state
        .settings
        .iter()
        .map(|(id, settings)| (settings.name.as_str(), *id))
        .collect();
    let mut announcements = Vec::new();
    for row in &rows {
        // A BufferInfo carrying a NetworkId the client cannot name is worse
        // than an absence; the history reappears the moment the network
        // returns to config.
        let Some(network) = by_name.get(row.network_name.as_str()).copied() else {
            continue;
        };
        state
            .buffers
            .entry(row.id)
            .or_insert((network, row.name.clone()));
        announcements.push(BufferInfo {
            id: row.id,
            network,
            name: row.name.clone(),
            kind: row.kind,
            last_read_seq: row.last_read_seq,
        });
    }
    if announcements.is_empty() {
        return;
    }
    let Some(lane) = bus.lane(client) else {
        return;
    };
    tokio::spawn(async move {
        for buffer in announcements {
            if lane
                .send(Directed::Event(Event::BufferCreated { buffer }))
                .await
                .is_err()
            {
                return;
            }
        }
    });
}

pub(super) fn wire(buffer: BufferId, message: crate::storage::StoredMessage) -> havoc_ipc::Message {
    havoc_ipc::Message {
        buffer,
        seq: message.seq,
        kind: message.kind,
        nick: message.nick,
        text: message.text.unwrap_or_default(),
        server_time: message.server_time,
        tags: message.tags,
    }
}
