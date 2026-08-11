//! What crosses the storage thread's channel, in both directions: the job
//! inputs a caller hands over and the outcomes that come back. Pure data — no
//! SQL, no connection, no thread — split from mod.rs for the size ratchet, and
//! the grouping is exactly "the vocabulary of the channel", which is what makes
//! `NetworkRow`'s id-space fence and the row types' core-privacy legible in one
//! place.

use std::collections::BTreeMap;

use havoc_ipc::{BufferId, BufferKind, MessageKind, NetworkId, Seq, ServerTime};

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
    /// Migration 0001's column: **one marker per buffer for the whole
    /// machine**, NULL until something sets it. Written by `Job::SetReadMarker`
    /// (prompt 9b) and handed to every attaching client; per-client markers are
    /// a stage-6 schema change, not a field.
    pub last_read_seq: Option<Seq>,
}

/// A finished job behind the flush barrier, correlated back to the session that
/// caused it — reads, and the one write small enough to answer this way. One
/// reply lane for all of them, not one `mpsc` per verb: core grows a single
/// select arm.
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
    /// The read marker landed (or did not). `buffer`/`seq` ride along so core can
    /// broadcast `ReadMarkerChanged` without remembering the request.
    MarkerSet {
        client: crate::bus::ClientId,
        request: havoc_ipc::RequestId,
        buffer: BufferId,
        seq: Seq,
        /// `Err` is a client bug: an unknown buffer, or `seq < 1`.
        result: Result<(), String>,
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
