//! The storage thread's job loop and the SQL each job executes. Split from
//! mod.rs for size alone; the ownership story is unchanged — one thread, one
//! connection, jobs in, replies out.

use std::sync::mpsc;

use havoc_ipc::{BufferId, NetworkId};
#[cfg(test)]
use havoc_ipc::{Seq, ServerTime};
use rusqlite::Connection;

#[cfg(test)]
use super::RawMessage;
use super::{Job, StorageError, buffer_kind_str};
use havoc_ipc::BufferKind;

pub(super) fn run(conn: Connection, jobs: &mpsc::Receiver<Job>) {
    while let Ok(job) = jobs.recv() {
        match job {
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
            #[cfg(test)]
            Job::InsertRaw {
                buffer,
                message,
                reply,
            } => {
                let _ = reply.send(insert_raw(&conn, buffer, &message));
            }
            #[cfg(test)]
            Job::FetchRaw { buffer, seq, reply } => {
                let _ = reply.send(fetch_raw(&conn, buffer, seq));
            }
            Job::Shutdown => break,
        }
    }
}

fn ensure_network(conn: &Connection, name: &str) -> Result<NetworkId, StorageError> {
    conn.execute(
        "INSERT INTO network (name) VALUES (?1) ON CONFLICT (name) DO NOTHING",
        [name],
    )?;
    let id = conn.query_row("SELECT id FROM network WHERE name = ?1", [name], |row| {
        row.get(0)
    })?;
    Ok(NetworkId(id))
}

fn ensure_buffer(
    conn: &Connection,
    network: NetworkId,
    name: &str,
    kind: BufferKind,
) -> Result<BufferId, StorageError> {
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

#[cfg(test)]
fn insert_raw(
    conn: &Connection,
    buffer: BufferId,
    message: &RawMessage,
) -> Result<(), StorageError> {
    conn.execute(
        "INSERT INTO message (buffer_id, seq, server_time, kind, nick, text)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
        (
            buffer.0,
            message.seq.0,
            message.server_time.as_unix_millis(),
            message.kind_code,
            message.nick.as_deref(),
            message.text.as_deref(),
        ),
    )?;
    Ok(())
}

#[cfg(test)]
fn fetch_raw(
    conn: &Connection,
    buffer: BufferId,
    seq: Seq,
) -> Result<Option<RawMessage>, StorageError> {
    let mut stmt = conn.prepare(
        "SELECT seq, kind, nick, text, server_time FROM message
         WHERE buffer_id = ?1 AND seq = ?2",
    )?;
    let mut rows = stmt.query((buffer.0, seq.0))?;
    let Some(row) = rows.next()? else {
        return Ok(None);
    };
    Ok(Some(RawMessage {
        seq: Seq(row.get(0)?),
        kind_code: row.get(1)?,
        nick: row.get(2)?,
        text: row.get(3)?,
        server_time: ServerTime::from_unix_millis(row.get(4)?),
    }))
}
