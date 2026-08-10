//! Framing and transport implementations (in-process mpsc, UDS later) and the
//! trait both sides code against — per NORTH-STAR §4.2 and its naming
//! amendment (Supernaut app, havoc engine). No business logic of any kind.
//!
//! One impl exists, so the trait is deliberately not dyn-compatible; if
//! stage 4 wants runtime selection it wraps impls in an enum rather than
//! boxing. `Incoming` is transport-local — whether a framed union needs a
//! wire type in havoc-ipc is stage 4's call.

use havoc_ipc::{Event, Request, Response};
use tokio::sync::mpsc;

/// Everything a client can receive: its responses and events (broadcast or
/// directed — the distinction is the core's; by here they are one stream).
#[derive(Debug, Clone)]
pub enum Incoming {
    Response(Response),
    Event(Event),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TransportError {
    /// The peer is gone; no more messages will arrive.
    Closed,
    /// The client fell behind the broadcast lane and missed `n` events. Loud
    /// by design: a projection that silently skipped events is undebuggable.
    Lagged(u64),
}

impl std::fmt::Display for TransportError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Closed => write!(f, "transport closed"),
            Self::Lagged(n) => write!(f, "transport lagged: missed {n} events"),
        }
    }
}

impl std::error::Error for TransportError {}

/// The client-side transport contract.
pub trait ClientTransport {
    fn send(&mut self, request: Request) -> impl Future<Output = Result<(), TransportError>>;
    fn recv(&mut self) -> impl Future<Output = Result<Incoming, TransportError>>;
}

/// The embedded-mode transport: typed values over tokio channels, no
/// serialization anywhere (§4.3 — same messages, different pipe).
pub struct InProcess {
    pub requests: mpsc::Sender<Request>,
    pub incoming: mpsc::Receiver<Result<Incoming, TransportError>>,
}

impl ClientTransport for InProcess {
    async fn send(&mut self, request: Request) -> Result<(), TransportError> {
        self.requests
            .send(request)
            .await
            .map_err(|_| TransportError::Closed)
    }

    async fn recv(&mut self) -> Result<Incoming, TransportError> {
        match self.incoming.recv().await {
            Some(result) => result,
            None => Err(TransportError::Closed),
        }
    }
}
