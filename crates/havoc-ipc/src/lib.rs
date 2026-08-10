//! Wire types shared across the havoc engine and its clients — requests,
//! responses, events, IDs — per NORTH-STAR §4.2 and its naming amendment
//! (Supernaut app, havoc engine). Near zero-dep by rule: data, not logic.
//!
//! Everything here crosses the client/core boundary and is serde-serializable.
//! Storage rows, actor state, and anything else that never crosses the wire
//! belongs in `havoc-core`, not here.

pub mod caps;

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

/// Bumped on breaking wire changes. The stage-4 capability handshake
/// negotiates from this plus the constants in [`caps`].
pub const PROTOCOL_VERSION: u32 = 1;

/// A configured IRC network. Aligned with the storage layer's `INTEGER`
/// primary keys.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct NetworkId(pub i64);

/// A buffer: channel, query, server console, or special.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct BufferId(pub i64);

/// Monotonic per-buffer sequence number, assigned by the storage layer at
/// insert. **The only ordering key** (NORTH-STAR §4.6): it is ours, it never
/// changes, and it is correct even when everything else lies.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct Seq(pub i64);

/// Correlates a [`Request`] with its exactly-one [`Response`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct RequestId(pub u64);

/// The wall-clock time a message claims to have happened (IRCv3 `server-time`
/// tag, or local receipt time as fallback), in Unix milliseconds.
///
/// Used only for display and for merging history batches. Deliberately
/// implements **neither `Ord` nor `PartialOrd`** and never will: server clocks
/// are wrong, `CHATHISTORY` batches arrive out of order, and bouncers replay.
/// Ordering is [`Seq`]'s job alone (NORTH-STAR §4.6, §6.1) — this type exists
/// so a sort by timestamp fails to compile.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ServerTime(i64);

impl ServerTime {
    pub fn from_unix_millis(millis: i64) -> Self {
        Self(millis)
    }

    /// For display and persistence. Not for sorting; see the type docs.
    pub fn as_unix_millis(self) -> i64 {
        self.0
    }
}

/// What kind of line a message is. Matches the storage `kind` column's needs
/// so the storage layer does not invent a second enum.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MessageKind {
    Privmsg,
    Notice,
    Join,
    Part,
    Quit,
    Mode,
    Topic,
    Nick,
    Server,
}

/// What kind of buffer a buffer is (NORTH-STAR §4.9).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BufferKind {
    Channel,
    Query,
    Server,
    Special,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct NetworkInfo {
    pub id: NetworkId,
    pub name: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BufferInfo {
    pub id: BufferId,
    pub network: NetworkId,
    pub name: String,
    pub kind: BufferKind,
    pub last_read_seq: Option<Seq>,
}

/// One line as it crosses the wire — in `MessageAdded` events, backlog
/// responses, and search results alike.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Message {
    pub buffer: BufferId,
    pub seq: Seq,
    pub kind: MessageKind,
    /// Absent for server/system lines.
    pub nick: Option<String>,
    pub text: String,
    pub server_time: ServerTime,
    /// Remaining IRCv3 message-tags, unescaped, minus those already lifted
    /// into fields (`msgid`, `server-time`).
    pub tags: BTreeMap<String, String>,
}

/// Where a backlog window is anchored (NORTH-STAR §4.7). There is deliberately
/// no "give me the buffer" variant, ever.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Anchor {
    Before(Seq),
    After(Seq),
    Latest,
    AroundSearchHit(Seq),
}

/// A client-to-core request. Exactly one [`Response`] comes back, correlated
/// by [`RequestId`].
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Request {
    pub id: RequestId,
    pub body: RequestBody,
}

/// The stage-1 request surface.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum RequestBody {
    Connect {
        network: NetworkId,
    },
    Join {
        network: NetworkId,
        channel: String,
    },
    SendText {
        buffer: BufferId,
        text: String,
    },
    /// Windowed, always. `limit` is capped server-side regardless of what the
    /// client asks for (NORTH-STAR §6.3).
    FetchBacklog {
        buffer: BufferId,
        anchor: Anchor,
        limit: u32,
    },
    /// Query syntax (structural filters like `from:`/`in:`) is parsed by the
    /// core; the wire carries it verbatim.
    Search {
        query: String,
    },
    SetReadMarker {
        buffer: BufferId,
        seq: Seq,
    },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Response {
    pub id: RequestId,
    pub body: ResponseBody,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ResponseBody {
    /// The request was accepted; any resulting state lands as [`Event`]s.
    Ack,
    Error {
        message: String,
    },
    Backlog {
        messages: Vec<Message>,
    },
}

/// Where a network's connection currently stands, as the client sees it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ConnectionPhase {
    Connecting,
    Registered,
    Disconnected,
}

/// Unsolicited core-to-client events. Most variants are broadcast to every
/// attached client; **request-correlated variants (`SearchResults`) travel a
/// per-session directed lane only** — broadcasting another client's search
/// hits would be an information leak (prompt-5 decision).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum Event {
    ConnectionState {
        network: NetworkId,
        phase: ConnectionPhase,
        /// Human-readable cause when the phase change has one (e.g. the
        /// fail-closed SASL reason behind a `Disconnected`). Added post-v1 of
        /// this type: `#[serde(default)]` keeps old encodings decodable, per
        /// the unknown-field tolerance the roundtrip tests prove.
        #[serde(default)]
        detail: Option<String>,
    },
    BufferCreated {
        buffer: BufferInfo,
    },
    MessageAdded {
        message: Message,
    },
    SearchResults {
        request: RequestId,
        hits: Vec<Message>,
    },
    ReadMarkerChanged {
        buffer: BufferId,
        seq: Seq,
    },
}
