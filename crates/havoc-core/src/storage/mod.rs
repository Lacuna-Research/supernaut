//! SQLite storage on a dedicated thread behind a channel (NORTH-STAR §6.6).
//!
//! One thread owns the `Connection`; everything else talks to it by sending a
//! [`Job`] and waiting on a reply channel. This is what keeps search (prompt 8)
//! from ever blocking a render loop, and it makes "nothing else ever touches
//! the connection" structural rather than conventional.
//!
//! The write path (prompt 7): the ingest lane is fire-and-forget into this
//! thread's unbounded job queue — history is never dropped for backpressure —
//! with seq assignment, msgid dedup (the partial unique index), and ~100ms
//! batched transactions all enforced here, where the single writer lives.

mod exec;
mod identity;
mod migrations;
mod query;

use std::fmt;
use std::path::Path;
use std::sync::mpsc;
use std::thread;

use exec::run;
use std::collections::BTreeMap;

use havoc_ipc::{Anchor, BufferId, BufferKind, MessageKind, NetworkId, Seq, ServerTime};
pub use identity::{buffer_kind_str, kind_code, kind_from_code};
pub use migrations::MigrationReport;
use rusqlite::{Connection, OpenFlags};

#[derive(Debug)]
pub enum StorageError {
    Sqlite(rusqlite::Error),
    /// CBOR-encoding the tags blob failed (should be impossible for a string
    /// map; loud rather than silent if it ever is not).
    Encode(String),
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
            Self::Encode(e) => write!(f, "encoding tags: {e}"),
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

/// A storage-row id for a network — deliberately a different type from the
/// wire's caller-assigned `NetworkId`, so the two id spaces can never be
/// swapped silently (they were both `1` in every test).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct NetworkRow(pub(crate) i64);

/// One classified line on its way to disk. Core-private: the wire `Message`
/// has no msgid berth, deliberately — dedup is storage's business.
#[derive(Debug, Clone)]
pub struct Ingest {
    /// Buffer name: channel, or peer nick for queries.
    pub target: String,
    pub kind: MessageKind,
    pub nick: Option<String>,
    pub account: Option<String>,
    pub text: Option<String>,
    pub server_time: ServerTime,
    /// Server msgid when tagged; storage synthesizes `fnv:<hex>` otherwise.
    pub msgid: Option<String>,
    /// Remaining tags, stored as CBOR (NULL when empty).
    pub tags: BTreeMap<String, String>,
}

/// A finished search, correlated back to its requester.
#[derive(Debug)]
pub struct SearchOutcome {
    pub client: crate::bus::ClientId,
    pub request: havoc_ipc::RequestId,
    /// Err carries the SQLite message (a malformed MATCH string is user
    /// input; it comes back as a Response::Error, never a hang).
    pub result: Result<Vec<(BufferId, StoredMessage)>, String>,
}

/// One row of the `buffer` table, for the attach-time announcement. Storage
/// rows stay core-private (prompt 3's fence): this carries the *network name*,
/// never a caller `NetworkId` — the row knows a `NetworkRow`, and minting a
/// wire id inside storage is the id-space confusion `NetworkRow` exists to
/// kill. Core maps the name back.
#[derive(Debug, Clone)]
pub struct BufferRow {
    pub id: BufferId,
    pub network_name: String,
    pub name: String,
    pub kind: BufferKind,
    /// Read from migration 0001's column. Nothing writes it until prompt 9b,
    /// so every value is NULL today — filling a field the wire already froze
    /// costs nothing and saves 9b a second pass through the read path.
    pub last_read_seq: Option<Seq>,
}

/// A finished read job, correlated back to the session that caused it. One
/// reply lane for both, not two: core grows a single select arm.
#[derive(Debug)]
pub enum ReadOutcome {
    Backlog {
        client: crate::bus::ClientId,
        request: havoc_ipc::RequestId,
        /// `Err` is a client bug (unknown buffer, zero limit); an empty window
        /// is `Ok(vec![])`, the end-of-scrollback signal.
        result: Result<Vec<(BufferId, StoredMessage)>, String>,
    },
    Buffers {
        client: crate::bus::ClientId,
        result: Result<Vec<BufferRow>, String>,
    },
}

/// What one ingest produced, reported after commit.
#[derive(Debug, Clone)]
pub struct IngestOutcome {
    pub network: NetworkId,
    pub buffer: BufferId,
    pub buffer_name: String,
    pub buffer_kind: BufferKind,
    /// True only when this ingest inserted the buffer row itself.
    pub buffer_created: bool,
    /// `None` for a dedup hit: no seq consumed, no event owed.
    pub message: Option<StoredMessage>,
}

#[derive(Debug, Clone)]
pub struct StoredMessage {
    pub seq: Seq,
    pub kind: MessageKind,
    pub nick: Option<String>,
    pub text: Option<String>,
    pub server_time: ServerTime,
    pub tags: BTreeMap<String, String>,
}

enum Job {
    SchemaVersion {
        reply: mpsc::Sender<i64>,
    },
    EnsureNetwork {
        name: String,
        reply: mpsc::Sender<Result<NetworkRow, StorageError>>,
    },
    EnsureBuffer {
        network: NetworkRow,
        name: String,
        kind: BufferKind,
        reply: mpsc::Sender<Result<BufferId, StorageError>>,
    },
    /// Search rides this same queue (read-your-writes: the run loop flushes
    /// pending ingests before any non-ingest job). Fire-and-forget like
    /// Ingest; the outcome carries the correlation back.
    Search {
        spec: crate::search::SearchSpec,
        client: crate::bus::ClientId,
        request: havoc_ipc::RequestId,
        reply: tokio::sync::mpsc::Sender<SearchOutcome>,
    },
    /// A backlog window. Rides the same queue behind the same flush barrier,
    /// so a window sees the line that landed a millisecond ago exactly as
    /// search does. Fire-and-forget like Search.
    Backlog {
        buffer: BufferId,
        anchor: Anchor,
        limit: u32,
        client: crate::bus::ClientId,
        request: havoc_ipc::RequestId,
        reply: tokio::sync::mpsc::Sender<ReadOutcome>,
    },
    /// Enumerate every buffer, for the attach-time announcement. No client can
    /// ask for this — §4.7 forbids "give me the buffer" and §4.5 makes the
    /// buffer set something the core *announces* instead.
    ListBuffers {
        client: crate::bus::ClientId,
        reply: tokio::sync::mpsc::Sender<ReadOutcome>,
    },
    /// The write path: fire-and-forget; the outcome arrives on the tokio
    /// sender after the batch commits.
    Ingest {
        network: NetworkId,
        row: NetworkRow,
        item: Ingest,
        outcome: tokio::sync::mpsc::Sender<IngestOutcome>,
    },
    Shutdown,
}

/// Clonable, `Send` handle to the storage thread — what async code moves into
/// `spawn_blocking` closures (the methods block on `recv`; never call them
/// inline on an executor thread).
#[derive(Clone)]
pub struct StorageClient {
    jobs: mpsc::Sender<Job>,
}

/// Owner of the storage thread. Dropping it shuts the thread down.
pub struct Storage {
    client: StorageClient,
    thread: Option<thread::JoinHandle<()>>,
}

impl Storage {
    /// Open (creating if absent) and migrate the database at `path`, then move
    /// the connection onto its dedicated thread. Migration runs on the caller's
    /// thread so failures surface before anything else starts.
    pub fn open(path: &Path, trace: bool) -> Result<(Self, MigrationReport), StorageError> {
        let mut conn = Connection::open_with_flags(
            path,
            OpenFlags::SQLITE_OPEN_READ_WRITE | OpenFlags::SQLITE_OPEN_CREATE,
        )?;
        conn.pragma_update(None, "journal_mode", "WAL")?;
        conn.pragma_update(None, "foreign_keys", "ON")?;
        // NORMAL under WAL: fsync at checkpoint, not per commit — committed
        // data survives an app crash; only the power-loss tail is conceded.
        // Chosen by prompt 7's flood measurement (numbers in BUILD-LOG).
        conn.pragma_update(None, "synchronous", "NORMAL")?;
        let report = migrations::migrate(&mut conn)?;

        let (jobs, rx) = mpsc::channel::<Job>();
        let thread = thread::Builder::new()
            .name("havoc-storage".to_owned())
            .spawn(move || run(conn, &rx, trace))
            .expect("spawning the storage thread");

        Ok((
            Self {
                client: StorageClient { jobs },
                thread: Some(thread),
            },
            report,
        ))
    }

    /// The one route to storage operations — "replace, don't deprecate":
    /// `Storage` owns the thread; `StorageClient` does the talking.
    pub fn client(&self) -> StorageClient {
        self.client.clone()
    }
}

impl StorageClient {
    pub fn schema_version(&self) -> Result<i64, StorageError> {
        let (reply, rx) = mpsc::channel();
        self.send(Job::SchemaVersion { reply })?;
        rx.recv().map_err(|_| StorageError::ThreadGone)
    }

    pub fn ensure_network(&self, name: &str) -> Result<NetworkRow, StorageError> {
        let (reply, rx) = mpsc::channel();
        self.send(Job::EnsureNetwork {
            name: name.to_owned(),
            reply,
        })?;
        rx.recv().map_err(|_| StorageError::ThreadGone)?
    }

    /// Fire-and-forget into the batch window; the outcome (buffer identity,
    /// seq, dedup verdict) arrives on `outcome` after commit. Non-blocking by
    /// design — this is the no-await ingest lane.
    pub fn ingest(
        &self,
        network: NetworkId,
        row: NetworkRow,
        item: Ingest,
        outcome: tokio::sync::mpsc::Sender<IngestOutcome>,
    ) -> Result<(), StorageError> {
        self.send(Job::Ingest {
            network,
            row,
            item,
            outcome,
        })
    }

    /// Fire-and-forget search on the storage thread's single connection —
    /// after the flush barrier, so a send followed by a search finds the line.
    pub fn search(
        &self,
        spec: crate::search::SearchSpec,
        client: crate::bus::ClientId,
        request: havoc_ipc::RequestId,
        reply: tokio::sync::mpsc::Sender<SearchOutcome>,
    ) -> Result<(), StorageError> {
        self.send(Job::Search {
            spec,
            client,
            request,
            reply,
        })
    }

    /// Fire-and-forget backlog window, behind the same flush barrier search
    /// uses. The window arrives on `reply` as a [`ReadOutcome::Backlog`].
    pub fn backlog(
        &self,
        buffer: BufferId,
        anchor: Anchor,
        limit: u32,
        client: crate::bus::ClientId,
        request: havoc_ipc::RequestId,
        reply: tokio::sync::mpsc::Sender<ReadOutcome>,
    ) -> Result<(), StorageError> {
        self.send(Job::Backlog {
            buffer,
            anchor,
            limit,
            client,
            request,
            reply,
        })
    }

    /// Fire-and-forget buffer enumeration for one attaching session.
    pub fn list_buffers(
        &self,
        client: crate::bus::ClientId,
        reply: tokio::sync::mpsc::Sender<ReadOutcome>,
    ) -> Result<(), StorageError> {
        self.send(Job::ListBuffers { client, reply })
    }

    pub fn ensure_buffer(
        &self,
        network: NetworkRow,
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

    fn send(&self, job: Job) -> Result<(), StorageError> {
        self.jobs.send(job).map_err(|_| StorageError::ThreadGone)
    }
}

impl Drop for Storage {
    fn drop(&mut self) {
        let _ = self.client.jobs.send(Job::Shutdown);
        if let Some(thread) = self.thread.take() {
            let _ = thread.join();
        }
    }
}

#[cfg(test)]
mod tests;
