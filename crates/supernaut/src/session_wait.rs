//! The debug CLI's whole waiting-and-draining story: what a `wait` verb watches,
//! the one place a request is registered and answered, and the drain `quit` and
//! stdin EOF share. Split from session.rs for the size ratchet, exactly as
//! session_print.rs and session_backlog.rs were.
//!
//! One structure, one insert site, one remove site: [`SessionState::outstanding`]
//! is filled by `request()` — which classifies the body itself, so a new verb
//! cannot forget to register — and drained here *before* the response body is
//! matched, so an `Error` still ends a wait with something printed.

use std::time::Duration;

use havoc_ipc::{ConnectionPhase, MessageKind, RequestBody};
use havoc_transport::{ClientTransport, Incoming, TransportError};

use crate::session::SessionState;
use crate::session_backlog::print_backlog;
use crate::session_print::print_event;

/// What a `wait` verb is waiting for, derived from the request body rather than
/// remembered by each call site. Deliberately total: an unclassified verb counts
/// as `Other` and still leaves `outstanding`, which is what the quit drain needs.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Awaited {
    Search,
    Backlog,
    Marker,
    Other,
}

impl From<&RequestBody> for Awaited {
    fn from(body: &RequestBody) -> Self {
        match body {
            RequestBody::Search { .. } => Self::Search,
            RequestBody::FetchBacklog { .. } => Self::Backlog,
            RequestBody::SetReadMarker { .. } => Self::Marker,
            RequestBody::Connect { .. }
            | RequestBody::Join { .. }
            | RequestBody::SendText { .. } => Self::Other,
        }
    }
}

/// Responses seen per verb class. `wait search|backlog|marker` count **responses**
/// rather than events: a request that fails comes back as an `Error` on the same
/// `RequestId`, so the wait ends with a printed error instead of a timeout with
/// nothing to read.
#[derive(Debug, Default)]
pub(crate) struct Answered {
    pub(crate) search: u64,
    pub(crate) backlog: u64,
    pub(crate) marker: u64,
}

/// Per-buffer `MessageAdded` tallies, kept apart deliberately: `chat` is what a
/// person means by "a message" (privmsg and notice), while `total` is what the
/// *store* counts — every kind, Joins included — and is therefore the one a claim
/// about `seq` must be made against.
#[derive(Debug, Default, Clone, Copy)]
pub(crate) struct MsgCounts {
    pub(crate) chat: u64,
    pub(crate) total: u64,
}

impl MsgCounts {
    pub(crate) fn record(&mut self, kind: MessageKind) {
        self.total += 1;
        if matches!(kind, MessageKind::Privmsg | MessageKind::Notice) {
            self.chat += 1;
        }
    }
}

fn arg<T: std::str::FromStr>(rest: &[&str], at: usize, default: T) -> T {
    rest.get(at).and_then(|s| s.parse().ok()).unwrap_or(default)
}

const TARGETS: &str = "'registered', 'buffer', 'message', 'rows', 'search', 'backlog', 'marker'";

/// `wait registered [secs] | buffer <name> [secs] | message|rows <name> [count]
/// [secs] | search|backlog|marker [count] [secs]`.
pub(crate) async fn wait_command(state: &mut SessionState, parts: &[&str]) -> Result<(), String> {
    let target = parts.first().copied().unwrap_or("");
    let rest = parts.get(1..).unwrap_or(&[]);
    let (name, count, secs) = match target {
        "registered" => (None, 1, arg(rest, 0, 10)),
        "buffer" => (rest.first().map(|s| (*s).to_owned()), 1, arg(rest, 1, 10)),
        "message" | "rows" => (
            rest.first().map(|s| (*s).to_owned()),
            arg(rest, 1, 1),
            arg(rest, 2, 10),
        ),
        "search" | "backlog" | "marker" => (None, arg(rest, 0, 1), arg(rest, 1, 10)),
        _ => {
            println!("error - wait knows {TARGETS}");
            return Ok(());
        }
    };
    wait(state, target, name, count, secs).await
}

/// Consume (and still print) everything arriving until the predicate holds or the
/// deadline names itself.
async fn wait(
    state: &mut SessionState,
    target: &str,
    name: Option<String>,
    count: u64,
    secs: u64,
) -> Result<(), String> {
    let satisfied = |state: &SessionState| match target {
        "registered" => state.phase == Some(ConnectionPhase::Registered),
        "buffer" => name
            .as_deref()
            .is_some_and(|n| state.buffers.contains_key(n)),
        "message" => counted(state, name.as_deref(), |c| c.chat) >= count,
        "rows" => counted(state, name.as_deref(), |c| c.total) >= count,
        "search" => state.answered.search >= count,
        "backlog" => state.answered.backlog >= count,
        "marker" => state.answered.marker >= count,
        _ => true,
    };

    let deadline = tokio::time::Instant::now() + Duration::from_secs(secs);
    while !satisfied(state) {
        match tokio::time::timeout_at(deadline, state.transport.recv()).await {
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

fn counted(state: &SessionState, name: Option<&str>, pick: impl Fn(&MsgCounts) -> u64) -> u64 {
    name.and_then(|n| state.buffers.get(n))
        .and_then(|id| state.msg_counts.get(id))
        .map_or(0, pick)
}

/// The drain both `quit` and stdin EOF go through: wait for every outstanding
/// request to be answered, printing as usual, then sweep for 50ms so an event
/// queued *behind* its own response (a `SearchResults`, a `ReadMarkerChanged`) is
/// printed rather than discarded by the runtime drop.
///
/// A timeout is an `Err`, naming the count: an engine that did not answer is a
/// finding, and the process must exit non-zero so the harness notices. The 50ms
/// is a quiet period on a queue we own, not a race papered over with a sleep —
/// zero would be racy against the adapter task's one hop.
pub(crate) async fn finish(state: &mut SessionState, secs: u64) -> Result<(), String> {
    let deadline = tokio::time::Instant::now() + Duration::from_secs(secs);
    while !state.outstanding.is_empty() {
        match tokio::time::timeout_at(deadline, state.transport.recv()).await {
            Ok(incoming) => handle_incoming(state, incoming)?,
            Err(_) => {
                return Err(format!(
                    "quit: {} request(s) unanswered after {secs}s",
                    state.outstanding.len()
                ));
            }
        }
    }
    let sweep = tokio::time::Instant::now() + Duration::from_millis(50);
    while let Ok(incoming) = tokio::time::timeout_at(sweep, state.transport.recv()).await {
        // A transport closing mid-sweep is not a failure: quit is underway.
        if handle_incoming(state, incoming).is_err() {
            break;
        }
    }
    Ok(())
}

/// The one remove site. The counter is bumped **before** the body match on
/// purpose: an `Error` must end a wait too, or a failed verb becomes a timeout
/// with nothing printed.
pub(crate) fn handle_incoming(
    state: &mut SessionState,
    incoming: Result<Incoming, TransportError>,
) -> Result<(), String> {
    match incoming {
        Ok(Incoming::Response(response)) => {
            match state.outstanding.remove(&response.id) {
                Some(Awaited::Search) => state.answered.search += 1,
                Some(Awaited::Backlog) => state.answered.backlog += 1,
                Some(Awaited::Marker) => state.answered.marker += 1,
                Some(Awaited::Other) | None => {}
            }
            match response.body {
                havoc_ipc::ResponseBody::Ack => println!("ok {}", response.id.0),
                havoc_ipc::ResponseBody::Error { message } => {
                    println!("error {} {message}", response.id.0);
                }
                havoc_ipc::ResponseBody::Backlog { messages } => {
                    print_backlog(response.id, &messages);
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

#[cfg(test)]
mod tests {
    use super::*;
    use havoc_ipc::{Request, RequestId};
    use havoc_transport::InProcess;
    use std::collections::HashMap;
    use tokio::sync::mpsc;

    /// The drain's **failure** path, which nothing else exercises: a `finish()` that
    /// returned `Ok` on timeout would let the entire live-run acceptance suite pass
    /// while `quit` silently discarded in-flight requests — exactly the prompt-7
    /// regression this machinery exists to close, and invisible from the outside
    /// because the assertions count printed responses, not the exit code.
    ///
    /// Deadline 0 is the degenerate case on purpose: it keeps the test instant
    /// without a paused-clock runtime, and it is the same code path a real 10s
    /// timeout takes.
    #[tokio::test]
    async fn finish_reports_unanswered_requests_rather_than_returning_ok() {
        let (requests, _req_rx) = mpsc::channel::<Request>(4);
        // Held, not dropped: a closed lane would surface as `transport closed`
        // instead of the timeout this test is about.
        let (_in_tx, incoming) = mpsc::channel(4);
        let mut state = SessionState {
            transport: InProcess { requests, incoming },
            network: havoc_ipc::NetworkId(1),
            next_request: 2,
            buffers: HashMap::new(),
            msg_counts: HashMap::new(),
            last_hits: HashMap::new(),
            outstanding: HashMap::from([(RequestId(1), Awaited::Marker)]),
            answered: Answered::default(),
            phase: None,
        };

        let error = finish(&mut state, 0)
            .await
            .expect_err("an unanswered request must not look like a clean quit");
        assert!(
            error.contains('1') && error.contains("unanswered"),
            "the error must name what is still owed: {error}"
        );
    }
}
