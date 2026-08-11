//! The event bus: two lanes, deliberately (prompt-5 decision).
//!
//! Broadcast events go to every attached client over a tokio broadcast
//! channel. Request-correlated events (`SearchResults`) go out a per-session
//! directed lane only — broadcasting one client's search hits to another is an
//! information leak. The lane choice is structural: [`Bus::broadcast`] refuses
//! correlated variants in debug builds.
//!
//! The directed lane has exactly one delivery primitive, and it never awaits
//! (prompt-9b decision, superseding `try_direct`): no path in the core loop can
//! be stalled by a client, so the storage thread's `blocking_send` on the
//! bounded reply lanes can never be held behind a reader.

use std::collections::HashMap;

use havoc_ipc::Event;
use tokio::sync::{broadcast, mpsc};

/// Identifies one attached session. Core-assigned; `RequestId` is
/// client-chosen and unique only within a session, so routing keys on this.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ClientId(pub u64);

/// Broadcast lane depth. A receiver that falls this far behind gets a loud
/// `Lagged` error rather than silently missing events — a client whose
/// projection quietly skipped events is the §4.5 bug class that is
/// undebuggable later.
pub const EVENT_CHANNEL_CAPACITY: usize = 256;

/// Per-session directed lane depth. Chosen to sit far above any legitimate
/// attach replay (one message per buffer, and a human does not have 4096
/// buffers) and far below hurting a laptop for `BufferInfo`-sized traffic.
///
/// Honest about the bound: this counts **messages, not bytes**, and one message
/// can be a 200-row window. Byte-based accounting belongs with stage 4's socket
/// server, where clients become plural and untrusted.
pub const DIRECTED_LANE_CAPACITY: usize = 4096;

/// What a session receives on its directed lane: its own responses and its
/// correlated events. (Core cannot name havoc-transport's `Incoming` — no
/// crate edge exists — so this is core's own union; the binary adapts.)
#[derive(Debug, Clone)]
pub enum Directed {
    Response(havoc_ipc::Response),
    Event(Event),
}

pub struct Bus {
    broadcast: broadcast::Sender<Event>,
    directed: HashMap<ClientId, mpsc::Sender<Directed>>,
}

impl Bus {
    pub fn new() -> Self {
        let (broadcast, _) = broadcast::channel(EVENT_CHANNEL_CAPACITY);
        Self {
            broadcast,
            directed: HashMap::new(),
        }
    }

    pub fn subscribe_broadcast(&self) -> broadcast::Receiver<Event> {
        self.broadcast.subscribe()
    }

    /// For `CoreHandle`: new sessions subscribe without reaching the bus.
    pub fn broadcast_sender(&self) -> broadcast::Sender<Event> {
        self.broadcast.clone()
    }

    pub fn register(&mut self, id: ClientId, lane: mpsc::Sender<Directed>) {
        self.directed.insert(id, lane);
    }

    /// Broadcast to every attached client. Correlated variants are a
    /// programming error here — they belong on [`Bus::direct`]. The tell is
    /// structural: a correlated variant carries a `RequestId`.
    pub fn broadcast(&self, event: Event) {
        debug_assert!(
            !matches!(event, Event::SearchResults { .. }),
            "SearchResults is request-correlated and must go out directed"
        );
        // Zero receivers is fine (no clients attached); errors here carry no
        // other meaning.
        let _ = self.broadcast.send(event);
    }

    /// Deliver to exactly one session — **the** directed primitive, and it never
    /// awaits, never blocks, and never silently drops a message while the lane
    /// lives. Returns whether the message was queued.
    ///
    /// `Closed` removes the lane silently: the session detached, which is
    /// ordinary. `Full` at [`DIRECTED_LANE_CAPACITY`] removes it after one loud
    /// line naming the `ClientId` — a client that has ignored 4096 answers is
    /// broken, and dropping message 4097 quietly would leave it broken and
    /// undiagnosed. Dropping a *single* message instead of the session was
    /// rejected for that reason.
    ///
    /// Why synchronous rather than awaiting: with no await point here, **no path
    /// in the core loop can be stalled by a client**, so the storage thread's
    /// `blocking_send` on `search_tx`/`reads_tx` can never end up parked behind
    /// a wedged reader. It is also what makes an ordered pair (a Response and
    /// its correlated Event) indivisible: two calls with no await between them
    /// either both land or the lane is already gone and the session is over.
    ///
    /// A consequence worth knowing: removal only drops the `Sender`, so a killed
    /// session still drains what is already queued and *then* sees `Closed` — it
    /// observes everything up to the message that found the lane full, which the
    /// loud line names.
    pub fn direct(&mut self, id: ClientId, message: Directed) -> bool {
        let Some(lane) = self.directed.get(&id) else {
            return false;
        };
        match lane.try_send(message) {
            Ok(()) => true,
            Err(mpsc::error::TrySendError::Closed(_)) => {
                self.directed.remove(&id);
                false
            }
            Err(mpsc::error::TrySendError::Full(_)) => {
                eprintln!(
                    "client {} has not drained {DIRECTED_LANE_CAPACITY} directed messages; \
                     dropping the session",
                    id.0
                );
                self.directed.remove(&id);
                false
            }
        }
    }
}

impl Default for Bus {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use havoc_ipc::{ConnectionPhase, NetworkId, RequestId};

    fn response(id: u64) -> Directed {
        Directed::Response(havoc_ipc::Response {
            id: RequestId(id),
            body: havoc_ipc::ResponseBody::Ack,
        })
    }

    /// The capacity claim, exercised: a session that drains nothing keeps its
    /// lane for 4096 messages and loses it — the session, not one message — on
    /// the 4097th, loudly and once. Draining afterwards does not revive it,
    /// which is what distinguishes "removed" from "merely full".
    #[tokio::test]
    async fn direct_drops_a_session_that_fills_the_lane() {
        let mut bus = Bus::new();
        let (tx, mut rx) = mpsc::channel(DIRECTED_LANE_CAPACITY);
        bus.register(ClientId(1), tx);

        for slot in 0..DIRECTED_LANE_CAPACITY {
            assert!(
                bus.direct(ClientId(1), response(slot as u64)),
                "slot {slot} of {DIRECTED_LANE_CAPACITY} must be accepted"
            );
        }
        assert!(
            !bus.direct(ClientId(1), response(9_001)),
            "Full must be refused, never awaited"
        );
        // Everything already queued still drains — removal drops the Sender,
        // nothing more — and the lane is gone all the same.
        rx.recv().await.expect("queued messages still arrive");
        assert!(
            !bus.direct(ClientId(1), response(9_002)),
            "the lane is removed, not retried forever"
        );
    }

    /// A detached session is ordinary: no lane, no noise, no delivery.
    #[tokio::test]
    async fn direct_drops_a_closed_lane_silently() {
        let mut bus = Bus::new();
        let (tx, rx) = mpsc::channel(8);
        bus.register(ClientId(2), tx);
        drop(rx);

        assert!(!bus.direct(ClientId(2), response(1)));
        assert!(!bus.direct(ClientId(2), response(2)), "and stays gone");
    }

    /// The leak test the prompt orders: a second session must never observe
    /// the first's SearchResults, while broadcast events reach both.
    #[tokio::test]
    async fn directed_lane_does_not_leak_across_sessions() {
        let mut bus = Bus::new();
        let mut rx_a_broadcast = bus.subscribe_broadcast();
        let mut rx_b_broadcast = bus.subscribe_broadcast();
        let (tx_a, mut rx_a) = mpsc::channel(8);
        let (tx_b, mut rx_b) = mpsc::channel(8);
        bus.register(ClientId(1), tx_a);
        bus.register(ClientId(2), tx_b);

        bus.broadcast(Event::ConnectionState {
            network: NetworkId(1),
            phase: ConnectionPhase::Registered,
            detail: None,
        });
        assert!(bus.direct(
            ClientId(1),
            Directed::Event(Event::SearchResults {
                request: RequestId(7),
                hits: Vec::new(),
            }),
        ));

        assert!(matches!(
            rx_a_broadcast.recv().await,
            Ok(Event::ConnectionState { .. })
        ));
        assert!(matches!(
            rx_b_broadcast.recv().await,
            Ok(Event::ConnectionState { .. })
        ));
        assert!(matches!(
            rx_a.recv().await,
            Some(Directed::Event(Event::SearchResults { .. }))
        ));
        assert!(
            rx_b.try_recv().is_err(),
            "session B must not see session A's correlated event"
        );
    }
}
