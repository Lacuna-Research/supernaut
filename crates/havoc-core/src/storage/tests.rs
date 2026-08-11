//! Ingest/identity/search tests for the storage thread — split from mod.rs for
//! the size ratchet alone. Real temp-file SQLite throughout; a mock would test
//! the mock. The backlog windows and the read markers live in the `backlog` and
//! `markers` submodules, same reason; both reuse `item`/`drain` from here.
use super::*;
use havoc_ipc::{MessageKind, ServerTime};
use std::collections::BTreeMap;

fn temp_store() -> (std::path::PathBuf, Storage, StorageClient) {
    let dir = std::env::temp_dir().join(format!(
        "havoc-ingest-{}-{:p}",
        std::process::id(),
        &std::process::id() as *const _
    ));
    std::fs::create_dir_all(&dir).expect("temp dir");
    let path = dir.join("ingest.db");
    let _ = std::fs::remove_file(&path);
    let (storage, _) = Storage::open(&path, false).expect("open");
    let client = storage.client();
    (dir, storage, client)
}

fn item(msgid: Option<&str>, text: &str, millis: i64) -> Ingest {
    Ingest {
        target: "#supernaut".to_owned(),
        kind: MessageKind::Privmsg,
        nick: Some("alice".to_owned()),
        account: None,
        text: Some(text.to_owned()),
        server_time: ServerTime::from_unix_millis(millis),
        msgid: msgid.map(str::to_owned),
        tags: BTreeMap::new(),
    }
}

fn drain(rx: &mut tokio::sync::mpsc::Receiver<IngestOutcome>, n: usize) -> Vec<IngestOutcome> {
    let mut out = Vec::new();
    while out.len() < n {
        out.push(rx.blocking_recv().expect("outcome"));
    }
    out
}

/// Same msgid twice → one row, one event; dedup consumes no seq. The
/// entire idempotency story hangs on ON CONFLICT DO NOTHING reporting
/// zero changed rows — this test is that assumption, exercised.
#[test]
fn msgid_dedup_yields_one_row_and_one_event() {
    let (dir, storage, client) = temp_store();
    let row = client.ensure_network("libera").expect("network");
    let (tx, mut rx) = tokio::sync::mpsc::channel(16);

    client
        .ingest(
            NetworkId(1),
            row,
            item(Some("m1"), "hello", 1_000),
            tx.clone(),
        )
        .expect("send");
    client
        .ingest(
            NetworkId(1),
            row,
            item(Some("m1"), "hello", 1_000),
            tx.clone(),
        )
        .expect("send");
    client
        .ingest(NetworkId(1), row, item(Some("m2"), "again", 2_000), tx)
        .expect("send");

    let outcomes = drain(&mut rx, 3);
    assert!(outcomes[0].buffer_created, "first touch creates the buffer");
    assert_eq!(outcomes[0].message.as_ref().expect("inserted").seq, Seq(1));
    assert!(outcomes[1].message.is_none(), "duplicate msgid: no event");
    assert!(!outcomes[1].buffer_created);
    assert_eq!(
        outcomes[2].message.as_ref().expect("inserted").seq,
        Seq(2),
        "dedup must not consume a seq"
    );

    drop(storage);
    let _ = std::fs::remove_dir_all(&dir);
}

/// Tagless identity: same (nick, text) in one 30s bucket collapses to one
/// row — §6.5's replay-safety trade, accepted and pinned.
#[test]
fn tagless_bucket_collapses_and_distinct_text_does_not() {
    let (dir, storage, client) = temp_store();
    let row = client.ensure_network("libera").expect("network");
    let (tx, mut rx) = tokio::sync::mpsc::channel(16);

    client
        .ingest(NetworkId(1), row, item(None, "hello", 1_000), tx.clone())
        .expect("send");
    client
        .ingest(NetworkId(1), row, item(None, "hello", 14_000), tx.clone())
        .expect("send");
    client
        .ingest(NetworkId(1), row, item(None, "different", 14_500), tx)
        .expect("send");

    let outcomes = drain(&mut rx, 3);
    assert!(outcomes[0].message.is_some());
    assert!(
        outcomes[1].message.is_none(),
        "same bucket, same content: collapsed"
    );
    assert!(outcomes[2].message.is_some(), "different text inserts");

    drop(storage);
    let _ = std::fs::remove_dir_all(&dir);
}

fn search_hits(client: &StorageClient, query: &str) -> Vec<(BufferId, StoredMessage)> {
    let spec = crate::search::parse(query).expect("query parses");
    let (tx, mut rx) = tokio::sync::mpsc::channel(4);
    client
        .search(spec, crate::bus::ClientId(1), havoc_ipc::RequestId(1), tx)
        .expect("send");
    rx.blocking_recv()
        .expect("outcome")
        .result
        .expect("search ok")
}

/// Trigger sync: an ingested line is immediately searchable; a
/// dedup-suppressed duplicate indexes nothing (ON CONFLICT DO NOTHING
/// does not fire the AFTER INSERT trigger — pinned here); NULL-text rows
/// never enter the index.
#[test]
fn fts_trigger_syncs_and_dedup_indexes_nothing() {
    let (dir, storage, client) = temp_store();
    let row = client.ensure_network("libera").expect("network");
    let (tx, mut rx) = tokio::sync::mpsc::channel(16);

    client
        .ingest(
            NetworkId(1),
            row,
            item(Some("m1"), "deployment failed", 1_000),
            tx.clone(),
        )
        .expect("send");
    client
        .ingest(
            NetworkId(1),
            row,
            item(Some("m1"), "deployment failed", 1_000),
            tx.clone(),
        )
        .expect("send");
    let mut join = item(Some("m2"), "x", 2_000);
    join.kind = MessageKind::Join;
    join.text = None;
    client.ingest(NetworkId(1), row, join, tx).expect("send");
    drain(&mut rx, 3);

    let hits = search_hits(&client, "deployment");
    assert_eq!(hits.len(), 1, "one row, one index entry, despite the dup");
    assert_eq!(hits[0].1.seq, Seq(1));

    // The NULL-text Join row must be absent from the index — assert the
    // index's actual row count, not a query that couldn't match it anyway.
    let db_path = dir.join("ingest.db");
    drop(storage);
    let conn = rusqlite::Connection::open(&db_path).expect("inspect");
    let fts_rows: i64 = conn
        .query_row("SELECT COUNT(*) FROM message_fts", [], |r| r.get(0))
        .expect("count");
    assert_eq!(fts_rows, 1, "one text row indexed; NULL-text join absent");
    drop(conn);
    let _ = std::fs::remove_dir_all(&dir);
}

/// `in:` is a buffer-**name** filter with no network scope, so one channel name
/// unions histories across networks. Pinned rather than fixed (prompt 10a): the
/// union is real, the grammar for scoping it is stage 2's `/search`, and this
/// test is what turns "we know" into a failing test when that lands.
#[test]
fn in_filter_unions_one_buffer_name_across_networks() {
    let (dir, storage, client) = temp_store();
    let a = client.ensure_network("net-a").expect("network a");
    let b = client.ensure_network("net-b").expect("network b");
    assert_ne!(a.0, b.0, "two rows, or the test is vacuous");
    let (tx, mut rx) = tokio::sync::mpsc::channel(16);

    // Same target (`#supernaut`, from `item`), different network rows.
    client
        .ingest(
            NetworkId(1),
            a,
            item(Some("a1"), "unionable line", 1_000),
            tx.clone(),
        )
        .expect("send");
    client
        .ingest(
            NetworkId(2),
            b,
            item(Some("b1"), "unionable line", 2_000),
            tx,
        )
        .expect("send");
    drain(&mut rx, 2);

    let hits = search_hits(&client, "in:#supernaut unionable");
    assert_eq!(
        hits.len(),
        2,
        "in: is unscoped: both networks' #supernaut match"
    );
    let buffers: std::collections::BTreeSet<i64> = hits.iter().map(|(b, _)| b.0).collect();
    assert_eq!(
        buffers.len(),
        2,
        "two distinct buffer rows sharing one name"
    );

    drop(storage);
    let _ = std::fs::remove_dir_all(&dir);
}

/// The rollback story: a mid-batch FK failure rolls the whole batch —
/// message rows, FTS rows, and the writer caches — and a clean re-ingest
/// starts at seq 1 and is searchable.
#[test]
fn mid_batch_failure_leaves_no_phantom_fts_rows() {
    let (dir, storage, client) = temp_store();
    let row = client.ensure_network("libera").expect("network");
    let (tx, mut rx) = tokio::sync::mpsc::channel(16);

    client
        .ingest(
            NetworkId(1),
            row,
            item(Some("g1"), "good line", 1_000),
            tx.clone(),
        )
        .expect("send");
    // Bogus network row: trips the buffer FK inside the same batch.
    client
        .ingest(
            NetworkId(1),
            NetworkRow(9_999),
            item(Some("b1"), "bad line", 1_500),
            tx.clone(),
        )
        .expect("send");
    // The failed batch sends no outcomes; a schema_version round-trip
    // forces the flush (any non-ingest job drains the batch) — a
    // deterministic barrier, not a sleep.
    let _ = client.schema_version();
    assert!(
        search_hits(&client, "good").is_empty(),
        "whole batch rolled back"
    );
    assert!(
        search_hits(&client, "bad").is_empty(),
        "no phantom FTS rows"
    );

    client
        .ingest(NetworkId(1), row, item(Some("g1"), "good line", 1_000), tx)
        .expect("send");
    let outcomes = drain(&mut rx, 1);
    assert_eq!(
        outcomes[0].message.as_ref().expect("inserted").seq,
        Seq(1),
        "caches reseeded from disk after rollback"
    );
    assert_eq!(search_hits(&client, "good").len(), 1);

    drop(storage);
    let _ = std::fs::remove_dir_all(&dir);
}

/// The server-side cap: 150 matches in, 100 hits out (§6.3's posture).
#[test]
fn search_caps_at_one_hundred_hits() {
    let (dir, storage, client) = temp_store();
    let row = client.ensure_network("libera").expect("network");
    let (tx, mut rx) = tokio::sync::mpsc::channel(256);
    for i in 0..150 {
        client
            .ingest(
                NetworkId(1),
                row,
                item(Some(&format!("m{i}")), &format!("needle {i}"), 1_000 + i),
                tx.clone(),
            )
            .expect("send");
    }
    drain(&mut rx, 150);
    assert_eq!(search_hits(&client, "needle").len(), 100);

    drop(storage);
    let _ = std::fs::remove_dir_all(&dir);
}

/// The migrations-first payoff, pinned: a version-1 database written
/// before FTS existed upgrades on open and its rows become searchable
/// via the backfill.
#[test]
fn v1_database_upgrades_and_backfills() {
    let dir = std::env::temp_dir().join(format!("havoc-v1up-{}", std::process::id()));
    std::fs::create_dir_all(&dir).expect("temp dir");
    let path = dir.join("old.db");
    let _ = std::fs::remove_file(&path);
    {
        // Craft a v1 file with raw SQL: migration 0001's shape only.
        let conn = rusqlite::Connection::open(&path).expect("open raw");
        conn.execute_batch(include_str!("../../migrations/0001_init.sql"))
            .expect("v1 schema");
        conn.execute_batch("PRAGMA user_version = 1").expect("v1");
        conn.execute_batch(
            "INSERT INTO network (id, name) VALUES (1, 'old');
                 INSERT INTO buffer (id, network_id, name, kind)
                    VALUES (1, 1, '#old', 'channel');
                 INSERT INTO message (buffer_id, seq, msgid, server_time, kind, nick, text)
                    VALUES (1, 1, 'a', 1000, 0, 'alice', 'ancient deployment lore');",
        )
        .expect("v1 rows");
    }
    let (storage, report) = Storage::open(&path, false).expect("upgrade");
    assert_eq!(report.from_version, 1);
    assert_eq!(report.to_version, 2);
    let hits = search_hits(&storage.client(), "ancient");
    assert_eq!(hits.len(), 1, "backfill made pre-FTS history searchable");

    drop(storage);
    let _ = std::fs::remove_dir_all(&dir);
}

mod backlog;
mod markers;
