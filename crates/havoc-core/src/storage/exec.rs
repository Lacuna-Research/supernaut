//! The storage thread's job loop, the batched write path, and the SQL each
//! job executes. One thread, one connection, jobs in, replies out — and the
//! only writer, which is what makes the cached per-buffer seq counters sound.
//!
//! Batching (§6.5): pending ingests flush as one transaction at
//! [`MAX_BATCH_ROWS`], at ~[`BATCH_WINDOW`] after the first pending row, when
//! any non-ingest job arrives (reads must see writes), and at Shutdown —
//! queue order puts pending ingests ahead of the Shutdown job, so quit loses
//! nothing.

use std::collections::HashMap;
use std::sync::mpsc;
use std::time::{Duration, Instant};

use havoc_ipc::{BufferId, BufferKind, NetworkId, Seq};
use rusqlite::Connection;

use super::identity::{parse_kind, synthetic_msgid};
use super::query::{run_backlog, run_list_buffers, run_search};
use super::rows::{ensure_buffer, ensure_network, set_read_marker};
use super::{
    Ingest, IngestOutcome, Job, NetworkRow, ReadOutcome, SearchOutcome, StorageError,
    StoredMessage, buffer_kind_str, kind_code,
};

const MAX_BATCH_ROWS: usize = 256;
const BATCH_WINDOW: Duration = Duration::from_millis(100);

struct PendingIngest {
    network: NetworkId,
    row: NetworkRow,
    item: Ingest,
    outcome: tokio::sync::mpsc::Sender<IngestOutcome>,
}

struct WriterState {
    /// (network row, buffer name) → buffer. Sound: this thread is the only
    /// writer.
    buffers: HashMap<(i64, String), (BufferId, BufferKind)>,
    /// Next seq per buffer, seeded from MAX(seq) on first touch.
    next_seq: HashMap<i64, i64>,
    trace: bool,
}

pub(super) fn run(conn: Connection, jobs: &mpsc::Receiver<Job>, trace: bool) {
    let mut state = WriterState {
        buffers: HashMap::new(),
        next_seq: HashMap::new(),
        trace,
    };
    let mut pending: Vec<PendingIngest> = Vec::new();
    let mut deadline: Option<Instant> = None;

    loop {
        let job = match deadline {
            None => match jobs.recv() {
                Ok(job) => Some(job),
                Err(_) => break,
            },
            Some(when) => match jobs.recv_timeout(when.saturating_duration_since(Instant::now())) {
                Ok(job) => Some(job),
                Err(mpsc::RecvTimeoutError::Timeout) => None,
                Err(mpsc::RecvTimeoutError::Disconnected) => break,
            },
        };

        match job {
            None => {
                flush(&conn, &mut state, &mut pending);
                deadline = None;
            }
            Some(Job::Ingest {
                network,
                row,
                item,
                outcome,
            }) => {
                pending.push(PendingIngest {
                    network,
                    row,
                    item,
                    outcome,
                });
                if pending.len() >= MAX_BATCH_ROWS {
                    flush(&conn, &mut state, &mut pending);
                    deadline = None;
                } else if deadline.is_none() {
                    deadline = Some(Instant::now() + BATCH_WINDOW);
                }
            }
            Some(other) => {
                // Reads must see writes: drain the batch before any other job.
                flush(&conn, &mut state, &mut pending);
                deadline = None;
                match other {
                    Job::SchemaVersion { reply } => {
                        let version = conn
                            .query_row("PRAGMA user_version", [], |row| row.get(0))
                            .unwrap_or(-1);
                        let _ = reply.send(version);
                    }
                    Job::EnsureNetwork { name, reply } => {
                        let _ = reply.send(ensure_network(&conn, &name));
                    }
                    Job::EnsureBuffer {
                        network,
                        name,
                        kind,
                        reply,
                    } => {
                        let _ = reply.send(ensure_buffer(&conn, network, &name, kind));
                    }
                    Job::Search {
                        spec,
                        client,
                        request,
                        reply,
                    } => {
                        let result = run_search(&conn, &spec);
                        let _ = reply.blocking_send(SearchOutcome {
                            client,
                            request,
                            result,
                        });
                    }
                    Job::Backlog {
                        buffer,
                        anchor,
                        limit,
                        client,
                        request,
                        reply,
                    } => {
                        let result = run_backlog(&conn, buffer, anchor, limit);
                        if state.trace {
                            let rows = result.as_ref().map_or(0, Vec::len);
                            eprintln!("storage backlog buffer={} rows={rows}", buffer.0);
                        }
                        let _ = reply.blocking_send(ReadOutcome::Backlog {
                            client,
                            request,
                            result,
                        });
                    }
                    Job::SetReadMarker {
                        buffer,
                        seq,
                        client,
                        request,
                        reply,
                    } => {
                        let result = set_read_marker(&conn, buffer, seq);
                        let _ = reply.blocking_send(ReadOutcome::MarkerSet {
                            client,
                            request,
                            buffer,
                            seq,
                            result,
                        });
                    }
                    Job::ListBuffers { client, reply } => {
                        let result = run_list_buffers(&conn);
                        let _ = reply.blocking_send(ReadOutcome::Buffers { client, result });
                    }
                    Job::Ingest { .. } => unreachable!("handled above"),
                    Job::Shutdown => return,
                }
            }
        }
    }
    // Channel gone with rows still pending: write them before dying.
    flush(&conn, &mut state, &mut pending);
}

/// One transaction for the whole batch, outcomes sent after commit in order.
fn flush(conn: &Connection, state: &mut WriterState, pending: &mut Vec<PendingIngest>) {
    if pending.is_empty() {
        return;
    }
    let batch = std::mem::take(pending);
    match write_batch(conn, state, &batch) {
        Ok(outcomes) => {
            if state.trace {
                let rows = outcomes.iter().filter(|(_, o)| o.message.is_some()).count();
                eprintln!("storage commit rows={rows}");
            }
            for (sender, outcome) in outcomes {
                let _ = sender.blocking_send(outcome);
            }
        }
        Err(error) => {
            // History write failure is loud and unswallowed; the rows are
            // reported lost rather than silently dropped.
            eprintln!("storage: batch of {} failed: {error}", batch.len());
        }
    }
}

type Outcomes = Vec<(tokio::sync::mpsc::Sender<IngestOutcome>, IngestOutcome)>;

fn write_batch(
    conn: &Connection,
    state: &mut WriterState,
    batch: &[PendingIngest],
) -> Result<Outcomes, StorageError> {
    conn.execute_batch("BEGIN")?;
    let result = (|| -> Result<Outcomes, StorageError> {
        let mut outcomes: Outcomes = Vec::with_capacity(batch.len());
        for pending in batch {
            let (buffer, kind, created) =
                ensure_buffer_cached(conn, state, pending.row, &pending.item.target)?;

            // Identity (§4.6/§6.4): server msgid when present, else the
            // synthetic content hash — one column, one unique index, both
            // identities.
            let msgid = pending.item.msgid.clone().unwrap_or_else(|| {
                synthetic_msgid(
                    pending.item.nick.as_deref(),
                    pending.item.text.as_deref(),
                    pending.item.server_time.as_unix_millis(),
                )
            });

            let next = match state.next_seq.get(&buffer.0) {
                Some(next) => *next,
                None => {
                    let max: i64 = conn.query_row(
                        "SELECT COALESCE(MAX(seq), 0) FROM message WHERE buffer_id = ?1",
                        [buffer.0],
                        |row| row.get(0),
                    )?;
                    max + 1
                }
            };

            let tags_blob = if pending.item.tags.is_empty() {
                None
            } else {
                let mut blob = Vec::new();
                ciborium::into_writer(&pending.item.tags, &mut blob)
                    .map_err(|e| StorageError::Encode(e.to_string()))?;
                Some(blob)
            };

            let changed = conn.execute(
                "INSERT INTO message
                   (buffer_id, seq, msgid, server_time, kind, nick, account, text, tags)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)
                 ON CONFLICT (buffer_id, msgid) WHERE msgid IS NOT NULL DO NOTHING",
                (
                    buffer.0,
                    next,
                    &msgid,
                    pending.item.server_time.as_unix_millis(),
                    kind_code(pending.item.kind),
                    pending.item.nick.as_deref(),
                    pending.item.account.as_deref(),
                    pending.item.text.as_deref(),
                    tags_blob.as_deref(),
                ),
            )?;

            let message = if changed == 1 {
                state.next_seq.insert(buffer.0, next + 1);
                Some(StoredMessage {
                    seq: Seq(next),
                    kind: pending.item.kind,
                    nick: pending.item.nick.clone(),
                    text: pending.item.text.clone(),
                    server_time: pending.item.server_time,
                    tags: pending.item.tags.clone(),
                })
            } else {
                // Duplicate: no seq consumed, no event emitted — idempotency
                // is the index's doing, not an application-layer memory.
                None
            };

            outcomes.push((
                pending.outcome.clone(),
                IngestOutcome {
                    network: pending.network,
                    buffer,
                    buffer_name: pending.item.target.clone(),
                    buffer_kind: kind,
                    buffer_created: created,
                    message,
                },
            ));
        }
        Ok(outcomes)
    })();

    match result {
        Ok(outcomes) => {
            conn.execute_batch("COMMIT")?;
            Ok(outcomes)
        }
        Err(error) => {
            let _ = conn.execute_batch("ROLLBACK");
            // The caches were mutated inside the rolled-back transaction:
            // phantom buffer ids and advanced seqs would poison every later
            // batch (reviewer catch). Drop them; they reseed from disk.
            state.buffers.clear();
            state.next_seq.clear();
            Err(error)
        }
    }
}

/// The buffer kind an ingest target implies. Deterministic per name, so the
/// insert path can never re-kind an existing buffer.
fn target_kind(target: &str) -> BufferKind {
    if target.starts_with('#') || target.starts_with('&') {
        BufferKind::Channel
    } else {
        BufferKind::Query
    }
}

fn ensure_buffer_cached(
    conn: &Connection,
    state: &mut WriterState,
    row: NetworkRow,
    target: &str,
) -> Result<(BufferId, BufferKind, bool), StorageError> {
    let key = (row.0, target.to_owned());
    if let Some((buffer, kind)) = state.buffers.get(&key) {
        return Ok((*buffer, *kind, false));
    }
    let wanted = target_kind(target);
    let created = conn.execute(
        "INSERT INTO buffer (network_id, name, kind) VALUES (?1, ?2, ?3)
         ON CONFLICT (network_id, name) DO NOTHING",
        (row.0, target, buffer_kind_str(wanted)),
    )? == 1;
    let (id, kind_str): (i64, String) = conn.query_row(
        "SELECT id, kind FROM buffer WHERE network_id = ?1 AND name = ?2",
        (row.0, target),
        |r| Ok((r.get(0)?, r.get(1)?)),
    )?;
    let stored = parse_kind(&kind_str);
    if stored != wanted && !created {
        // Kind is immutable after creation; a mismatch is loud, never
        // swallowed and never rewritten (prompt-3 note discharged).
        eprintln!(
            "storage: buffer {target} exists as {kind_str}, ingest implied {}; keeping {kind_str}",
            buffer_kind_str(wanted)
        );
    }
    state.buffers.insert(key, (BufferId(id), stored));
    Ok((BufferId(id), stored, created))
}
