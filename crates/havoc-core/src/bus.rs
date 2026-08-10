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
