//! The read queries against the storage thread's connection — search (prompt 8)
//! and the windowed backlog (prompt 9a), split from exec.rs for the size
//! ratchet; same connection, same flush barrier, same rules.

use havoc_ipc::{Anchor, BufferId, Seq};
use rusqlite::Connection;

use super::identity::parse_kind;
use super::{BufferRow, StoredMessage, kind_from_code};
use crate::search::SearchSpec;

/// Bounded like the backlog cap (§6.3): the wire has no has-more berth, so a
/// full window means "refine the query"; the CLI's hits count keeps
/// truncation visible.
const SEARCH_MAX_HITS: usize = 100;

/// The §6.3 fence made concrete: whatever a client asks for, a window is at
/// most this many rows — two screens at any sane terminal height, tens of KB on
/// the wire. Core-side on purpose: a client that compiles the cap in is a
/// client that breaks when it moves, and discovery is the stage-4 handshake's
/// job.
const BACKLOG_MAX_LIMIT: usize = 200;

/// The column list every message-row read shares, so [`hydrate`] can index
/// positionally without each query restating the order.
const MESSAGE_COLUMNS: &str = "buffer_id, seq, kind, nick, text, server_time, tags";

/// One `message` row exactly as SQLite hands it over.
type RawRow = (
    BufferId,
    i64,
    i64,
    Option<String>,
    Option<String>,
    i64,
    Option<Vec<u8>>,
);

fn raw_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<RawRow> {
    Ok((
        BufferId(row.get(0)?),
        row.get(1)?,
        row.get(2)?,
        row.get(3)?,
        row.get(4)?,
        row.get(5)?,
        row.get(6)?,
    ))
}

/// Row → core-private message, shared by search and backlog: `Ok(None)` is a
/// row whose kind code this build cannot name, skipped loudly. Shared
/// deliberately — this is the one block where `kind_from_code`'s
/// loud-on-unknown behaviour could fork silently between the two readers.
fn hydrate(raw: RawRow) -> Result<Option<(BufferId, StoredMessage)>, String> {
    let (buffer, seq, code, nick, text, millis, tags_blob) = raw;
    let Some(kind) = kind_from_code(code) else {
        eprintln!("storage: unknown kind code {code} in buffer {}", buffer.0);
        return Ok(None);
    };
    let tags = match tags_blob {
        None => std::collections::BTreeMap::new(),
        Some(blob) => {
            ciborium::from_reader(blob.as_slice()).map_err(|e| format!("tags blob: {e}"))?
        }
    };
    Ok(Some((
        buffer,
        StoredMessage {
            seq: Seq(seq),
            kind,
            nick,
            text,
            server_time: havoc_ipc::ServerTime::from_unix_millis(millis),
            tags,
        },
    )))
}

pub(super) fn run_search(
    conn: &Connection,
    spec: &SearchSpec,
) -> Result<Vec<(BufferId, StoredMessage)>, String> {
    let mut stmt = conn
        .prepare(
            "SELECT m.buffer_id, m.seq, m.kind, m.nick, m.text, m.server_time, m.tags
             FROM message_fts
             JOIN message m
               ON m.buffer_id = message_fts.buffer_id AND m.seq = message_fts.seq
             WHERE message_fts MATCH ?1
               AND (?2 IS NULL OR m.nick = ?2 COLLATE NOCASE)
               AND (?3 IS NULL OR m.buffer_id IN
                    (SELECT id FROM buffer WHERE name = ?3 COLLATE NOCASE))
               AND (?4 IS NULL OR m.server_time >= ?4)
               AND (?5 IS NULL OR m.server_time < ?5)
             ORDER BY rank
             LIMIT ?6",
        )
        .map_err(|e| e.to_string())?;
    let rows = stmt
        .query_map(
            rusqlite::params![
                spec.match_query,
                spec.nick,
                spec.buffer,
                spec.after,
                spec.before,
                i64::try_from(SEARCH_MAX_HITS).expect("fits"),
            ],
            raw_row,
        )
        .map_err(|e| e.to_string())?;

    let mut hits = Vec::new();
    for row in rows {
        if let Some(hit) = hydrate(row.map_err(|e| e.to_string())?)? {
            hits.push(hit);
        }
    }
    Ok(hits)
}

/// One backlog window, **always ascending by seq** for every anchor (§4.6: seq
/// is the only order, and one order for all four anchors means the client's
/// scroll math has no cases).
///
/// `Err` is reserved for a client bug: an unknown buffer, or `limit == 0`. No
/// rows in range is `Ok(vec![])` — the normal end-of-scrollback signal a client
/// pages until, and the two must stay distinguishable or a client bug looks
/// like end-of-scrollback forever.
pub(super) fn run_backlog(
    conn: &Connection,
    buffer: BufferId,
    anchor: Anchor,
    limit: u32,
) -> Result<Vec<(BufferId, StoredMessage)>, String> {
    if limit == 0 {
        return Err("backlog limit must be at least 1".to_owned());
    }
    let known: bool = conn
        .query_row(
            "SELECT EXISTS (SELECT 1 FROM buffer WHERE id = ?1)",
            [buffer.0],
            |row| row.get(0),
        )
        .map_err(|e| e.to_string())?;
    if !known {
        return Err(format!("unknown buffer {}", buffer.0));
    }

    // The single site that decides how many rows any window may bind, so every
    // later caller inherits the cap.
    let limit = (limit as usize).min(BACKLOG_MAX_LIMIT);
    match anchor {
        Anchor::Latest => scan(conn, buffer, "<=", Seq(i64::MAX), true, limit),
        // Exclusive: the client is holding s and asked for what precedes it.
        Anchor::Before(seq) => scan(conn, buffer, "<", seq, true, limit),
        Anchor::After(seq) => scan(conn, buffer, ">", seq, false, limit),
        Anchor::AroundSearchHit(seq) => {
            // The hit plus floor((n-1)/2) rows before and the remainder after.
            // A short side never grows the other, so the hit stays centred even
            // at a buffer edge and one cheap After window fills the rest. A seq
            // that no longer exists yields the neighbours around the gap rather
            // than an error — history is append-only today, but retention
            // (stage 6) will make gaps real.
            let before = (limit - 1) / 2;
            let after = (limit - 1) - before;
            let mut window = scan(conn, buffer, "<=", seq, true, before + 1)?;
            window.extend(scan(conn, buffer, ">", seq, false, after)?);
            Ok(window)
        }
    }
}

/// One range scan against `message`'s (buffer_id, seq) `WITHOUT ROWID` primary
/// key — the table *is* the index, which is why the windowed API costs nothing
/// (§6.3). A descending scan is reversed in Rust so callers only ever see
/// ascending rows.
fn scan(
    conn: &Connection,
    buffer: BufferId,
    op: &'static str,
    bound: Seq,
    descending: bool,
    limit: usize,
) -> Result<Vec<(BufferId, StoredMessage)>, String> {
    if limit == 0 {
        return Ok(Vec::new());
    }
    // Interpolated fragments are `&'static str` from this module alone; every
    // number the caller supplied is still a bound parameter.
    let sql = format!(
        "SELECT {MESSAGE_COLUMNS} FROM message
         WHERE buffer_id = ?1 AND seq {op} ?2
         ORDER BY seq {} LIMIT ?3",
        if descending { "DESC" } else { "ASC" }
    );
    let mut stmt = conn.prepare(&sql).map_err(|e| e.to_string())?;
    let rows = stmt
        .query_map(
            rusqlite::params![buffer.0, bound.0, i64::try_from(limit).expect("fits")],
            raw_row,
        )
        .map_err(|e| e.to_string())?;
    let mut out = Vec::new();
    for row in rows {
        if let Some(message) = hydrate(row.map_err(|e| e.to_string())?)? {
            out.push(message);
        }
    }
    if descending {
        out.reverse();
    }
    Ok(out)
}

/// Every buffer this database knows, for the attach-time announcement (§4.5).
/// Returns the *network name*, never a wire `NetworkId`: minting one inside
/// storage is exactly the id-space confusion `NetworkRow` exists to kill.
pub(super) fn run_list_buffers(conn: &Connection) -> Result<Vec<BufferRow>, String> {
    let mut stmt = conn
        .prepare(
            "SELECT b.id, n.name, b.name, b.kind, b.last_read_seq
             FROM buffer b JOIN network n ON n.id = b.network_id
             ORDER BY b.id",
        )
        .map_err(|e| e.to_string())?;
    let rows = stmt
        .query_map([], |row| {
            Ok(BufferRow {
                id: BufferId(row.get(0)?),
                network_name: row.get(1)?,
                name: row.get(2)?,
                kind: parse_kind(&row.get::<_, String>(3)?),
                last_read_seq: row.get::<_, Option<i64>>(4)?.map(Seq),
            })
        })
        .map_err(|e| e.to_string())?;
    rows.collect::<rusqlite::Result<Vec<_>>>()
        .map_err(|e| e.to_string())
}
