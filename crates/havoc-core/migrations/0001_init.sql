-- Initial schema, per NORTH-STAR §4.9 and the 2026-08-09 decisions:
-- buffer identity is per-network (UNIQUE(network_id, name); merged views are
-- query-side projections), and server_time is unix MILLISECONDS to match
-- havoc-ipc's ServerTime exactly — seconds would truncate IRCv3 server-time.
--
-- No FTS here: search arrives as its own migration (prompt 8), which is the
-- point of having migrations before data.

CREATE TABLE network (
    id   INTEGER PRIMARY KEY,
    name TEXT NOT NULL UNIQUE
);

CREATE TABLE buffer (
    id            INTEGER PRIMARY KEY,
    network_id    INTEGER NOT NULL REFERENCES network(id),
    name          TEXT NOT NULL,
    kind          TEXT NOT NULL, -- havoc-ipc BufferKind, snake_case
    last_read_seq INTEGER,
    UNIQUE (network_id, name)
);

CREATE TABLE message (
    buffer_id   INTEGER NOT NULL REFERENCES buffer(id),
    seq         INTEGER NOT NULL, -- monotonic per buffer, ours, the only order
    msgid       TEXT,             -- IRCv3, nullable, dedup only
    server_time INTEGER NOT NULL, -- unix millis; display/merge only, never order
    kind        INTEGER NOT NULL, -- havoc-ipc MessageKind via storage::kind_code
    nick        TEXT,
    account     TEXT,
    text        TEXT,
    tags        BLOB,             -- CBOR, remaining message-tags
    PRIMARY KEY (buffer_id, seq)
) WITHOUT ROWID;

CREATE UNIQUE INDEX msg_msgid ON message (buffer_id, msgid) WHERE msgid IS NOT NULL;
CREATE INDEX msg_time ON message (buffer_id, server_time);
