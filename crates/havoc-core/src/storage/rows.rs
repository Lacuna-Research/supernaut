//! The `buffer`/`network` row writes that are **not** part of the batched ingest
//! path: one statement each, answered immediately. Split from exec.rs for the
//! size ratchet, and the grouping is the one the single-writer rule implies —
//! these are the writes that stand alone, so they belong together and away from
//! the batch machinery.

use havoc_ipc::{BufferId, BufferKind, Seq};
use rusqlite::Connection;

use super::{NetworkRow, buffer_kind_str};

pub(super) fn ensure_network(
    conn: &Connection,
    name: &str,
) -> Result<NetworkRow, super::StorageError> {
    conn.execute(
        "INSERT INTO network (name) VALUES (?1) ON CONFLICT (name) DO NOTHING",
        [name],
    )?;
    let id = conn.query_row("SELECT id FROM network WHERE name = ?1", [name], |row| {
        row.get(0)
    })?;
    Ok(NetworkRow(id))
}

pub(super) fn ensure_buffer(
    conn: &Connection,
    network: NetworkRow,
    name: &str,
    kind: BufferKind,
) -> Result<BufferId, super::StorageError> {
    conn.execute(
        "INSERT INTO buffer (network_id, name, kind) VALUES (?1, ?2, ?3)
         ON CONFLICT (network_id, name) DO NOTHING",
        (network.0, name, buffer_kind_str(kind)),
    )?;
    let id = conn.query_row(
        "SELECT id FROM buffer WHERE network_id = ?1 AND name = ?2",
        (network.0, name),
        |row| row.get(0),
    )?;
    Ok(BufferId(id))
}

/// Move a buffer's read marker. **Last write wins, with no clamp**: the client is
/// the authority on where a person has read to, and scrolling back to an unread
/// point is a real product action, so a marker may move *backward*. A monotonic
/// clamp in the engine would refuse a legitimate request with no way to report
/// it, and "highest wins" is precisely the last-write-wins reconciliation rule
/// the PLAN Still-open owns — deciding it inside this `UPDATE` is how that
/// question gets answered by accident.
///
/// `seq` is **not** checked for existence and never clamped to `MAX(seq)`: a
/// marker is a position, not a row reference, and retention (stage 6) will make
/// gaps real — the same reasoning `Anchor::AroundSearchHit` over a vanished seq
/// already ships with. `seq < 1` is still an error: asking to mark nothing read
/// is a client bug, and answering it politely hides it (the call `limit == 0`
/// got in 9a).
///
/// Unknown buffer is decided by the `UPDATE`'s own `changes()` rather than a
/// preceding `EXISTS` — one statement, precise, and cheaper than the read path's
/// check.
pub(super) fn set_read_marker(conn: &Connection, buffer: BufferId, seq: Seq) -> Result<(), String> {
    if seq.0 < 1 {
        return Err("read marker seq must be at least 1".to_owned());
    }
    let changed = conn
        .execute(
            "UPDATE buffer SET last_read_seq = ?2 WHERE id = ?1",
            (buffer.0, seq.0),
        )
        .map_err(|e| e.to_string())?;
    if changed == 0 {
        return Err(format!("unknown buffer {}", buffer.0));
    }
    Ok(())
}
