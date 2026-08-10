//! Backlog-window tests: the four anchors, the 200-row cap, and the two client
//! bugs that must never masquerade as an empty window. Split from tests.rs for
//! the size ratchet; `item` and `drain` come from the parent module.

use havoc_ipc::{Anchor, BufferId, NetworkId, Seq};

use super::{drain, item};
use crate::storage::{ReadOutcome, Storage, StorageClient};

/// A private store per test: the backlog anchors seed hundreds of rows and one
/// of them mutates the file from a second connection, so name collisions are
/// not something to leave to a stack address.
fn temp_store_named(tag: &str) -> (std::path::PathBuf, Storage, StorageClient) {
    let dir = std::env::temp_dir().join(format!("havoc-{tag}-{}", std::process::id()));
    std::fs::create_dir_all(&dir).expect("temp dir");
    let path = dir.join("h.db");
    let _ = std::fs::remove_file(&path);
    let (storage, _) = Storage::open(&path, false).expect("open");
    let client = storage.client();
    (dir, storage, client)
}

/// `n` rows into one buffer through the real ingest lane, so seqs are the ones
/// the write path would actually assign.
fn seed_rows(client: &StorageClient, n: usize) -> BufferId {
    let row = client.ensure_network("libera").expect("network");
    let (tx, mut rx) = tokio::sync::mpsc::channel(n + 1);
    for i in 1..=n {
        let text = format!("line {i}");
        client
            .ingest(
                NetworkId(1),
                row,
                item(Some(&format!("s{i}")), &text, 1_000 + i as i64),
                tx.clone(),
            )
            .expect("send");
    }
    drain(&mut rx, n)[0].buffer
}

fn window(
    client: &StorageClient,
    buffer: BufferId,
    anchor: Anchor,
    limit: u32,
) -> Result<Vec<Seq>, String> {
    let (tx, mut rx) = tokio::sync::mpsc::channel(4);
    client
        .backlog(
            buffer,
            anchor,
            limit,
            crate::bus::ClientId(1),
            havoc_ipc::RequestId(1),
            tx,
        )
        .expect("send");
    match rx.blocking_recv().expect("outcome") {
        ReadOutcome::Backlog { result, .. } => {
            result.map(|rows| rows.into_iter().map(|(_, m)| m.seq).collect())
        }
        other => panic!("expected a backlog outcome, got {other:?}"),
    }
}

/// Every anchor comes back ascending by seq (§4.6: one order for all four, so
/// the client's scroll math has no cases), Before/After are exclusive, and a
/// window off the end is a success — end of scrollback, not an error.
#[test]
fn backlog_anchors_are_ascending_and_exclusive() {
    let (dir, storage, client) = temp_store_named("backlog-anchors");
    let buffer = seed_rows(&client, 10);

    assert_eq!(
        window(&client, buffer, Anchor::Latest, 3).expect("latest"),
        vec![Seq(8), Seq(9), Seq(10)]
    );
    assert_eq!(
        window(&client, buffer, Anchor::Before(Seq(5)), 2).expect("before"),
        vec![Seq(3), Seq(4)],
        "Before is exclusive: the client already holds seq 5"
    );
    assert_eq!(
        window(&client, buffer, Anchor::After(Seq(5)), 2).expect("after"),
        vec![Seq(6), Seq(7)],
        "After is exclusive likewise"
    );
    assert!(
        window(&client, buffer, Anchor::After(Seq(10)), 5)
            .expect("empty window is a success")
            .is_empty()
    );

    drop(storage);
    let _ = std::fs::remove_dir_all(&dir);
}

/// The hit stays centred, and at a buffer edge the short side does not grow the
/// other — predictable position math, and one cheap After window fills the rest.
#[test]
fn backlog_around_hit_stays_centred_at_both_edges() {
    let (dir, storage, client) = temp_store_named("backlog-around");
    let buffer = seed_rows(&client, 10);

    assert_eq!(
        window(&client, buffer, Anchor::AroundSearchHit(Seq(5)), 5).expect("centre"),
        vec![Seq(3), Seq(4), Seq(5), Seq(6), Seq(7)]
    );
    assert_eq!(
        window(&client, buffer, Anchor::AroundSearchHit(Seq(1)), 5).expect("start"),
        vec![Seq(1), Seq(2), Seq(3)],
        "a short before-side must not grow the after-side"
    );
    assert_eq!(
        window(&client, buffer, Anchor::AroundSearchHit(Seq(10)), 5).expect("end"),
        vec![Seq(8), Seq(9), Seq(10)],
        "and a short after-side must not grow the before-side"
    );

    drop(storage);
    let _ = std::fs::remove_dir_all(&dir);
}

/// A seq that no longer exists returns the neighbours around the gap rather
/// than an error. History is append-only today; retention (stage 6) makes gaps
/// real, and a client holding a stale search hit must still land near it.
#[test]
fn backlog_around_a_vanished_seq_returns_its_neighbours() {
    let (dir, storage, client) = temp_store_named("backlog-gap");
    let buffer = seed_rows(&client, 10);
    {
        // A second connection, only to punch the hole the API cannot: the
        // write path is append-only by design.
        let conn = rusqlite::Connection::open(dir.join("h.db")).expect("open");
        conn.execute(
            "DELETE FROM message WHERE buffer_id = ?1 AND seq = 5",
            [buffer.0],
        )
        .expect("delete");
    }

    assert_eq!(
        window(&client, buffer, Anchor::AroundSearchHit(Seq(5)), 5).expect("gap"),
        vec![Seq(2), Seq(3), Seq(4), Seq(6), Seq(7)]
    );

    drop(storage);
    let _ = std::fs::remove_dir_all(&dir);
}

/// §6.3's fence: whatever the client asks for, the engine binds at most 200.
#[test]
fn backlog_caps_at_two_hundred_rows() {
    let (dir, storage, client) = temp_store_named("backlog-cap");
    let buffer = seed_rows(&client, 250);

    let rows = window(&client, buffer, Anchor::Latest, u32::MAX).expect("capped");
    assert_eq!(rows.len(), 200);
    assert_eq!(rows[0], Seq(51), "the cap takes the newest 200, ascending");
    assert_eq!(rows[199], Seq(250));

    drop(storage);
    let _ = std::fs::remove_dir_all(&dir);
}

/// The two client bugs that must never masquerade as an empty window: asking
/// for nothing, and asking about a buffer that does not exist.
#[test]
fn backlog_rejects_zero_limit_and_unknown_buffer() {
    let (dir, storage, client) = temp_store_named("backlog-errors");
    let buffer = seed_rows(&client, 3);

    assert!(
        window(&client, buffer, Anchor::Latest, 0).is_err(),
        "asking for nothing is a client bug, not an empty window"
    );
    let unknown = window(&client, BufferId(9_999), Anchor::Latest, 5)
        .expect_err("an unknown buffer is an error");
    assert!(unknown.contains("unknown buffer 9999"), "{unknown}");

    drop(storage);
    let _ = std::fs::remove_dir_all(&dir);
}
