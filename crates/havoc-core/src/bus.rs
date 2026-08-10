//! The event bus: two lanes, deliberately (prompt-5 decision).
//!
//! Broadcast events go to every attached client over a tokio broadcast
//! channel. Request-correlated events (`SearchResults`) go out a per-session
//! directed lane only — broadcasting one client's search hits to another is an
//! information leak. The lane choice is structural: [`Bus::broadcast`] refuses
//! correlated variants in debug builds.

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

    /// A clone of one session's lane, for a short-lived task that must await
    /// delivery without the core loop or the storage thread waiting on it (the
    /// attach-time buffer announcement).
    pub fn lane(&self, id: ClientId) -> Option<mpsc::Sender<Directed>> {
        self.directed.get(&id).cloned()
    }

    /// Non-blocking delivery, for read answers. The asymmetry with the write
    /// path, stated plainly: writes are buffered without bound because a lost
    /// line is unrecoverable, reads are dropped because a read is by definition
    /// re-askable — the engine must never hold history behind a reader that has
    /// ignored 64 answers and asked for a 65th.
    ///
    /// `Closed` removes the lane silently (an ordinary detach). `Full` removes
    /// it after one loud line naming the client: a wedged reader is a bug in
    /// that client, and it stops being this engine's problem.
    pub fn try_direct(&mut self, id: ClientId, message: Directed) -> bool {
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
                    "client {} is not draining its directed lane; dropping it and this read",
                    id.0
                );
                self.directed.remove(&id);
                false
            }
        }
    }

    /// Broadcast to every attached client. Correlated variants are a
    /// programming error here — they belong on [`Bus::direct`].
    pub fn broadcast(&self, event: Event) {
        debug_assert!(
            !matches!(event, Event::SearchResults { .. }),
            "SearchResults is request-correlated and must go out directed"
        );
        // Zero receivers is fine (no clients attached); errors here carry no
        // other meaning.
        let _ = self.broadcast.send(event);
    }

    /// Deliver to exactly one session. A gone session is dropped silently —
    /// it detached; that is normal, not an error.
    pub async fn direct(&mut self, id: ClientId, message: Directed) {
        if let Some(lane) = self.directed.get(&id)
            && lane.send(message).await.is_err()
        {
            self.directed.remove(&id);
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

    /// A wedged reader loses its lane, loudly, and the read with it — never the
    /// other way round (the engine must not stall behind it).
    #[tokio::test]
    async fn try_direct_drops_a_full_lane_loudly() {
        let mut bus = Bus::new();
        let (tx, _rx) = mpsc::channel(1);
        bus.register(ClientId(1), tx);

        assert!(bus.try_direct(ClientId(1), response(1)), "the one slot");
        assert!(
            !bus.try_direct(ClientId(1), response(2)),
            "Full must be refused, not awaited"
        );
        assert!(
            bus.lane(ClientId(1)).is_none(),
            "a wedged lane is removed, not retried forever"
        );
        assert!(!bus.try_direct(ClientId(1), response(3)), "and stays gone");
    }

    /// A detached session is ordinary: no lane, no noise, no delivery.
    #[tokio::test]
    async fn try_direct_drops_a_closed_lane_silently() {
        let mut bus = Bus::new();
        let (tx, rx) = mpsc::channel(8);
        bus.register(ClientId(2), tx);
        drop(rx);

        assert!(!bus.try_direct(ClientId(2), response(1)));
        assert!(bus.lane(ClientId(2)).is_none());
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
        bus.direct(
            ClientId(1),
            Directed::Event(Event::SearchResults {
                request: RequestId(7),
                hits: Vec::new(),
            }),
        )
        .await;

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
