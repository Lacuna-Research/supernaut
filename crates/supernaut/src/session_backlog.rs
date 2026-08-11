//! The debug CLI's `backlog` verb: anchor parsing and window printing — split
//! from session.rs for the size ratchet, exactly as session_print.rs was.
//!
//! `wait backlog` counts **responses**, not events — the pattern every wait
//! target now shares; the bookkeeping lives in session_wait.rs.

use havoc_ipc::{Anchor, BufferId, Message, RequestBody, RequestId, Seq};

use crate::session::{SessionState, request};

/// `backlog <buffer> [anchor] [limit]`, resolving the buffer name through the
/// same name→BufferId projection `send` uses — which now includes buffers this
/// process only learned about from the attach-time announcement.
pub(crate) async fn request_backlog(
    state: &mut SessionState,
    parts: &[&str],
) -> Result<(), String> {
    let Some(name) = parts.first() else {
        println!("error - backlog requires a buffer name");
        return Ok(());
    };
    let Some(&buffer) = state.buffers.get(*name) else {
        println!("error - no buffer for {name} (join it, or attach over its history)");
        return Ok(());
    };
    let spec = parts.get(1).copied().unwrap_or("latest");
    let limit: u32 = parts.get(2).and_then(|s| s.parse().ok()).unwrap_or(50);
    let anchor = match parse_anchor(spec, buffer, state) {
        Ok(anchor) => anchor,
        Err(message) => {
            println!("error - {message}");
            return Ok(());
        }
    };
    // `request` records the id and its class, so *either* answer ends a
    // `wait backlog`.
    request(
        state,
        RequestBody::FetchBacklog {
            buffer,
            anchor,
            limit,
        },
    )
    .await?;
    Ok(())
}

/// `latest | before:<seq> | after:<seq> | around:<seq> | around-hit`.
fn parse_anchor(spec: &str, buffer: BufferId, state: &SessionState) -> Result<Anchor, String> {
    if spec == "latest" {
        return Ok(Anchor::Latest);
    }
    if spec == "around-hit" {
        // The newest search hit seen for this buffer, so the jump-to-context
        // flow AroundSearchHit exists for is the flow the harness exercises,
        // rather than a seq the script pasted in.
        return state
            .last_hits
            .get(&buffer)
            .copied()
            .map(Anchor::AroundSearchHit)
            .ok_or_else(|| format!("no search hit seen for buffer {} yet", buffer.0));
    }
    let (verb, value) = spec
        .split_once(':')
        .ok_or_else(|| format!("unknown anchor {spec}"))?;
    let seq = Seq(value
        .parse()
        .map_err(|_| format!("anchor {verb} needs a seq, got {value}"))?);
    match verb {
        "before" => Ok(Anchor::Before(seq)),
        "after" => Ok(Anchor::After(seq)),
        "around" => Ok(Anchor::AroundSearchHit(seq)),
        other => Err(format!("unknown anchor {other}")),
    }
}

/// One header line plus one line per message, in order. The count on the header
/// is how a capped window stays visible with no has-more berth on the wire.
pub(crate) fn print_backlog(request: RequestId, messages: &[Message]) {
    // Every row in a window shares one buffer; an empty window has none to
    // name, and that emptiness is itself the end-of-scrollback signal.
    let buffer = match messages.first() {
        Some(message) => message.buffer.0.to_string(),
        None => "-".to_owned(),
    };
    println!(
        "backlog request={} buffer={buffer} count={}",
        request.0,
        messages.len()
    );
    for message in messages {
        println!(
            "line buffer={} seq={} nick={} text={}",
            message.buffer.0,
            message.seq.0,
            message.nick.as_deref().unwrap_or("-"),
            message.text
        );
    }
}
