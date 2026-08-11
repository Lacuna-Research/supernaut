//! Greppable event printing for the debug session — split from session.rs
//! for the size ratchet. One line per event; the live-run asserts grep these.

use havoc_ipc::{ConnectionPhase, Event};

use crate::session::SessionState;

pub(crate) fn print_event(state: &mut SessionState, event: &Event) {
    match event {
        Event::ConnectionState {
            network,
            phase,
            detail,
        } => {
            let phase_str = match phase {
                ConnectionPhase::Connecting => "connecting",
                ConnectionPhase::Registered => "registered",
                ConnectionPhase::Disconnected => "disconnected",
            };
            state.phase = Some(*phase);
            match detail {
                Some(detail) => println!(
                    "event connection-state network={} phase={phase_str} detail={detail}",
                    network.0
                ),
                None => println!(
                    "event connection-state network={} phase={phase_str}",
                    network.0
                ),
            }
        }
        Event::BufferCreated { buffer } => {
            state.buffers.insert(buffer.name.clone(), buffer.id);
            // last_read is printed because the attach announcement is the only
            // place a marker set by another process becomes observable.
            println!(
                "event buffer-created buffer={} network={} name={} last_read={}",
                buffer.id.0,
                buffer.network.0,
                buffer.name,
                buffer
                    .last_read_seq
                    .map_or_else(|| "-".to_owned(), |seq| seq.0.to_string())
            );
        }
        Event::MessageAdded { message } => {
            state
                .msg_counts
                .entry(message.buffer)
                .or_default()
                .record(message.kind);
            println!(
                "event message-added buffer={} seq={}",
                message.buffer.0, message.seq.0
            );
        }
        // No counting here: `wait search` counts *responses* now, like every
        // other wait target, so a failed search ends its wait too.
        Event::SearchResults { request, hits } => {
            println!(
                "event search-results request={} hits={}",
                request.0,
                hits.len()
            );
            for hit in hits {
                // The *newest* hit per buffer is what `backlog <b> around-hit`
                // jumps to, so keep the greatest seq — hits arrive in rank
                // order, not seq order. Never cleared: across searches, the
                // newest hit ever seen for a buffer wins.
                let newest = state.last_hits.entry(hit.buffer).or_insert(hit.seq);
                *newest = (*newest).max(hit.seq);
                println!(
                    "hit buffer={} seq={} nick={} text={}",
                    hit.buffer.0,
                    hit.seq.0,
                    hit.nick.as_deref().unwrap_or("-"),
                    hit.text
                );
            }
        }
        Event::ReadMarkerChanged { buffer, seq } => {
            println!("event read-marker buffer={} seq={}", buffer.0, seq.0);
        }
    }
}
