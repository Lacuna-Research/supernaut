//! The search query against the storage thread's connection — split from
//! exec.rs for the size ratchet; same connection, same barrier, same rules.

use havoc_ipc::BufferId;
use rusqlite::Connection;

use super::{StoredMessage, kind_from_code};
use crate::search::SearchSpec;

/// Bounded like the backlog cap (§6.3): the wire has no has-more berth, so a
/// full window means "refine the query"; the CLI's hits count keeps
/// truncation visible.
const SEARCH_MAX_HITS: usize = 100;

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
            |row| {
                Ok((
                    BufferId(row.get(0)?),
                    row.get::<_, i64>(1)?,
                    row.get::<_, i64>(2)?,
                    row.get::<_, Option<String>>(3)?,
                    row.get::<_, Option<String>>(4)?,
                    row.get::<_, i64>(5)?,
                    row.get::<_, Option<Vec<u8>>>(6)?,
                ))
            },
        )
        .map_err(|e| e.to_string())?;

    let mut hits = Vec::new();
    for row in rows {
        let (buffer, seq, code, nick, text, millis, tags_blob) = row.map_err(|e| e.to_string())?;
        let Some(kind) = kind_from_code(code) else {
            eprintln!("storage: unknown kind code {code} in buffer {}", buffer.0);
            continue;
        };
        let tags = match tags_blob {
            None => std::collections::BTreeMap::new(),
            Some(blob) => {
                ciborium::from_reader(blob.as_slice()).map_err(|e| format!("tags blob: {e}"))?
            }
        };
        hits.push((
            buffer,
            StoredMessage {
                seq: havoc_ipc::Seq(seq),
                kind,
                nick,
                text,
                server_time: havoc_ipc::ServerTime::from_unix_millis(millis),
                tags,
            },
        ));
    }
    Ok(hits)
}
