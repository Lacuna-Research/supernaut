-- Full-text search (prompt 8, NORTH-STAR §7.1). A plain self-contained FTS5
-- table, NOT external-content: `message` is WITHOUT ROWID, so external
-- content's content_rowid contract cannot hold (prompt-3 carry-forward,
-- decided at prompt 8). Disk is the cheap resource; moving parts are not.
--
-- Sync is by trigger, deliberately: the FTS write is structurally part of the
-- statement that inserts the row — no future insert path can forget it, the
-- dedup path (ON CONFLICT DO NOTHING) never fires it, and rollback carries it
-- for free. No UPDATE/DELETE triggers: message is append-only until the
-- retention story lands its own migration (stage 6).

CREATE VIRTUAL TABLE message_fts USING fts5(
    text,
    buffer_id UNINDEXED,
    seq UNINDEXED,
    tokenize = 'unicode61 remove_diacritics 2'
);

CREATE TRIGGER message_ai
AFTER INSERT ON message
WHEN new.text IS NOT NULL
BEGIN
    INSERT INTO message_fts (text, buffer_id, seq)
    VALUES (new.text, new.buffer_id, new.seq);
END;

-- Backfill everything already written — the migrations-first payoff.
INSERT INTO message_fts (text, buffer_id, seq)
SELECT text, buffer_id, seq FROM message WHERE text IS NOT NULL;
