//! Delivery of read answers and the read-marker write's outcome: search hits,
//! backlog windows, the marker's Ack-plus-event pair, and the attach-time buffer
//! announcement. Split from core.rs for the size ratchet; the grouping is the
//! core loop's whole delivery story, and since prompt 9b it uses exactly one
//! lane discipline — [`Bus::direct`], which never awaits, so nothing here can
//! stall the select loop or the storage thread behind it.

use std::collections::HashMap;

use havoc_ipc::{BufferId, BufferInfo, Event, NetworkId, Response, ResponseBody};

use super::CoreState;
use crate::bus::{Bus, ClientId, Directed};
use crate::storage::{ReadOutcome, SearchOutcome};

/// Response first (Ack, or the SQLite error for a malformed MATCH — user
/// input never hangs and is never swallowed), then the correlated hits on
/// the directed lane only. Errors get no event.
///
/// Not async, deliberately: with **no await point between the two calls**, the
/// ordered pair is indivisible — either both messages land or the lane is
/// already gone and the session is over. That is the answer to "what does half a
/// delivered pair mean": it is unrepresentable.
pub(super) fn handle_search_outcome(bus: &mut Bus, outcome: SearchOutcome) {
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
            );
        }
        Ok(rows) => {
            bus.direct(
                client,
                Directed::Response(Response {
                    id: request,
                    body: ResponseBody::Ack,
                }),
            );
            let hits = rows
                .into_iter()
                .map(|(buffer, m)| wire(buffer, m))
                .collect();
            bus.direct(
                client,
                Directed::Event(Event::SearchResults { request, hits }),
            );
        }
    }
}

/// Everything that comes back from a job behind the flush barrier. Delivery is
/// [`Bus::direct`] throughout: a reader that has stopped draining loses its
/// session rather than stalling the select loop and the storage thread behind
/// it.
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
            bus.direct(client, Directed::Response(Response { id: request, body }));
        }
        // The marker's outcome is a pair on *different* lanes, and therefore
        // deliberately unordered relative to each other: `Ack` to the requester,
        // `ReadMarkerChanged` to everyone. A failed write is not state, so it
        // gets the Error response and no event — search's rule, same reason.
        ReadOutcome::MarkerSet {
            client,
            request,
            buffer,
            seq,
            result,
        } => match result {
            Ok(()) => {
                bus.direct(
                    client,
                    Directed::Response(Response {
                        id: request,
                        body: ResponseBody::Ack,
                    }),
                );
                bus.broadcast(Event::ReadMarkerChanged { buffer, seq });
            }
            Err(message) => {
                bus.direct(
                    client,
                    Directed::Response(Response {
                        id: request,
                        body: ResponseBody::Error { message },
                    }),
                );
            }
        },
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
/// Delivered **inline from the core loop** (prompt 9b): the replay used to be a
/// spawned task holding a lane clone, which made the map entry no longer the
/// only aliveness token — a task parked on a wedged lane kept it alive, so
/// whether a stalled client was killed loudly or lingered as a zombie was
/// nondeterministic. `Bus::direct` cannot block, so there is nothing left for a
/// task to buy. Ordering against broadcast traffic is still not guaranteed; the
/// contract is that announcements are **idempotent** — a duplicate is legal, a
/// missing one is not.
///
/// That contract holds only while the lane is live. If `direct` reports the lane
/// gone, the replay is abandoned mid-list: under `Full` the client has been dropped
/// by policy, so it is not a client owed a complete buffer set any more — it is a
/// dead session, and finishing the list into a removed lane would say otherwise.
fn announce(
    state: &mut CoreState,
    bus: &mut Bus,
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
    for buffer in announcements {
        if !bus.direct(client, Directed::Event(Event::BufferCreated { buffer })) {
            return;
        }
    }
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
