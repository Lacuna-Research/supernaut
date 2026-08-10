//! SQLite storage on a dedicated thread behind a channel (NORTH-STAR §6.6).
//!
//! One thread owns the `Connection`; everything else talks to it by sending a
//! [`Job`] and waiting on a reply channel. This is what keeps search (prompt 8)
//! from ever blocking a render loop, and it makes "nothing else ever touches
//! the connection" structural rather than conventional.
//!
//! Deliberately absent until prompt 7: seq assignment, msgid dedup, and write
//! batching — the crate-internal smoke jobs below take an explicit [`Seq`] so
//! the schema can be exercised without pre-deciding the write path.

mod migrations;

use std::fmt;
use std::path::Path;
use std::sync::mpsc;
use std::thread;

use havoc_ipc::{BufferId, BufferKind, MessageKind, NetworkId};
#[cfg(test)]
use havoc_ipc::{Seq, ServerTime};
pub use migrations::MigrationReport;
use rusqlite::{Connection, OpenFlags};

#[derive(Debug)]
pub enum StorageError {
    Sqlite(rusqlite::Error),
    /// The database was written by a newer schema than this build knows.
    FutureSchema {
        found: i64,
        supported: i64,
    },
    /// The storage thread is gone; the reply channel died.
    ThreadGone,
}

impl fmt::Display for StorageError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Sqlite(e) => write!(f, "sqlite: {e}"),
            Self::FutureSchema { found, supported } => write!(
                f,
                "database schema version {found} is newer than this build supports ({supported}); refusing to touch it"
            ),
            Self::ThreadGone => write!(f, "storage thread terminated unexpectedly"),
        }
    }
}

impl std::error::Error for StorageError {}

impl From<rusqlite::Error> for StorageError {
    fn from(e: rusqlite::Error) -> Self {
        Self::Sqlite(e)
    }
}

/// The stable `message.kind` column encoding of [`MessageKind`]. A deliberate
/// mapping, not a derive: the integers are a disk format, and reordering the
/// enum must not be able to silently rewrite history's meaning.
pub fn kind_code(kind: MessageKind) -> i64 {
    match kind {
        MessageKind::Privmsg => 0,
        MessageKind::Notice => 1,
        MessageKind::Join => 2,
        MessageKind::Part => 3,
        MessageKind::Quit => 4,
        MessageKind::Mode => 5,
        MessageKind::Topic => 6,
        MessageKind::Nick => 7,
        MessageKind::Server => 8,
    }
}

/// The stable `buffer.kind` column encoding of [`BufferKind`] — matches the
/// snake_case the wire uses, by choice recorded here rather than by accident
/// of a serde attribute elsewhere.
pub fn buffer_kind_str(kind: BufferKind) -> &'static str {
    match kind {
        BufferKind::Channel => "channel",
        BufferKind::Query => "query",
        BufferKind::Server => "server",
        BufferKind::Special => "special",
    }
}

/// A row as the smoke path reads it back. Test-only until prompt 7 builds the
/// real ingest path; wire messages are `havoc_ipc::Message` and rows stay
/// core-private (prompt 3 fence).
#[cfg(test)]
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct RawMessage {
    pub seq: Seq,
    pub kind_code: i64,
    pub nick: Option<String>,
    pub text: Option<String>,
    pub server_time: ServerTime,
}

enum Job {
    SchemaVersion {
        reply: mpsc::Sender<i64>,
    },
    EnsureNetwork {
        name: String,
        reply: mpsc::Sender<Result<NetworkId, StorageError>>,
    },
    EnsureBuffer {
        network: NetworkId,
        name: String,
        kind: BufferKind,
        reply: mpsc::Sender<Result<BufferId, StorageError>>,
    },
    /// Smoke-test insert with an explicit seq. Prompt 7 replaces callers of
    /// this with the real ingest path (seq assignment, dedup, batching).
    #[cfg(test)]
    InsertRaw {
        buffer: BufferId,
        message: RawMessage,
        reply: mpsc::Sender<Result<(), StorageError>>,
    },
    #[cfg(test)]
    FetchRaw {
        buffer: BufferId,
        seq: Seq,
        reply: mpsc::Sender<Result<Option<RawMessage>, StorageError>>,
    },
    Shutdown,
}

/// Handle to the storage thread. Dropping it shuts the thread down.
pub struct Storage {
    jobs: mpsc::Sender<Job>,
    thread: Option<thread::JoinHandle<()>>,
}

impl Storage {
    /// Open (creating if absent) and migrate the database at `path`, then move
    /// the connection onto its dedicated thread. Migration runs on the caller's
    /// thread so failures surface before anything else starts.
    pub fn open(path: &Path) -> Result<(Self, MigrationReport), StorageError> {
        let mut conn = Connection::open_with_flags(
            path,
            OpenFlags::SQLITE_OPEN_READ_WRITE | OpenFlags::SQLITE_OPEN_CREATE,
        )?;
        conn.pragma_update(None, "journal_mode", "WAL")?;
        conn.pragma_update(None, "foreign_keys", "ON")?;
        // Deliberately no `synchronous` tuning here: prompt 7's flood harness
        // measures fsync behavior and owns that decision.
        let report = migrations::migrate(&mut conn)?;

        let (jobs, rx) = mpsc::channel::<Job>();
        let thread = thread::Builder::new()
            .name("havoc-storage".to_owned())
            .spawn(move || run(conn, &rx))
            .expect("spawning the storage thread");

        Ok((
            Self {
                jobs,
                thread: Some(thread),
            },
            report,
        ))
    }

    pub fn schema_version(&self) -> Result<i64, StorageError> {
        let (reply, rx) = mpsc::channel();
        self.send(Job::SchemaVersion { reply })?;
        rx.recv().map_err(|_| StorageError::ThreadGone)
    }

    pub fn ensure_network(&self, name: &str) -> Result<NetworkId, StorageError> {
        let (reply, rx) = mpsc::channel();
        self.send(Job::EnsureNetwork {
            name: name.to_owned(),
            reply,
        })?;
        rx.recv().map_err(|_| StorageError::ThreadGone)?
    }

    pub fn ensure_buffer(
        &self,
        network: NetworkId,
        name: &str,
        kind: BufferKind,
    ) -> Result<BufferId, StorageError> {
        let (reply, rx) = mpsc::channel();
        self.send(Job::EnsureBuffer {
            network,
            name: name.to_owned(),
            kind,
            reply,
        })?;
        rx.recv().map_err(|_| StorageError::ThreadGone)?
    }

    #[cfg(test)]
    pub(crate) fn insert_raw(
        &self,
        buffer: BufferId,
        message: RawMessage,
    ) -> Result<(), StorageError> {
        let (reply, rx) = mpsc::channel();
        self.send(Job::InsertRaw {
            buffer,
            message,
            reply,
        })?;
        rx.recv().map_err(|_| StorageError::ThreadGone)?
    }

    #[cfg(test)]
    pub(crate) fn fetch_raw(
        &self,
        buffer: BufferId,
        seq: Seq,
    ) -> Result<Option<RawMessage>, StorageError> {
        let (reply, rx) = mpsc::channel();
        self.send(Job::FetchRaw { buffer, seq, reply })?;
        rx.recv().map_err(|_| StorageError::ThreadGone)?
    }

    fn send(&self, job: Job) -> Result<(), StorageError> {
        self.jobs.send(job).map_err(|_| StorageError::ThreadGone)
    }
}

impl Drop for Storage {
    fn drop(&mut self) {
        let _ = self.jobs.send(Job::Shutdown);
        if let Some(thread) = self.thread.take() {
            let _ = thread.join();
        }
    }
}

fn run(conn: Connection, jobs: &mpsc::Receiver<Job>) {
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

#[cfg(test)]
mod tests {
    use super::*;
    use havoc_ipc::MessageKind;

    /// The smoke insert/read the prompt orders: explicit seq (assignment is
    /// prompt 7's job), one row through the thread and back, byte-identical.
    #[test]
    fn smoke_insert_and_read_through_the_channel() {
        let dir = std::env::temp_dir().join(format!("havoc-smoke-{}", std::process::id()));
        std::fs::create_dir_all(&dir).expect("temp dir");
        let path = dir.join("smoke.db");
        let _ = std::fs::remove_file(&path);

        let (storage, _) = Storage::open(&path).expect("open");
        let network = storage.ensure_network("libera").expect("network");
        let buffer = storage
            .ensure_buffer(network, "#supernaut", BufferKind::Channel)
            .expect("buffer");

        let sent = RawMessage {
            seq: Seq(1),
            kind_code: kind_code(MessageKind::Privmsg),
            nick: Some("alice".to_owned()),
            text: Some("hello".to_owned()),
            server_time: ServerTime::from_unix_millis(1_754_700_000_000),
        };
        storage.insert_raw(buffer, sent.clone()).expect("insert");

        let read = storage
            .fetch_raw(buffer, Seq(1))
            .expect("fetch")
            .expect("present");
        assert_eq!(read, sent);
        assert!(storage.fetch_raw(buffer, Seq(2)).expect("fetch").is_none());

        drop(storage);
        let _ = std::fs::remove_dir_all(&dir);
    }
}
