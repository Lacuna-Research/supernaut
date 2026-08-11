//! Read-marker tests: the write lands in the column the attach announcement
//! reads, a marker may move backward, and the two client bugs that must not be
//! answered politely. Split from tests.rs for the size ratchet, exactly as
//! `backlog` was; `item` and `drain` come from the parent module.

use havoc_ipc::{BufferId, NetworkId, Seq};

use super::{drain, item, temp_store};
use crate::storage::{BufferRow, ReadOutcome, StorageClient};

/// The read marker as a *write*: it lands in the column the announcement reads,
/// and `run_list_buffers` hands it back. One value per buffer for the whole
/// machine — that is what the single nullable column can represent.
#[test]
fn read_marker_writes_the_column_and_is_read_back() {
    let (dir, storage, client) = temp_store();
    let row = client.ensure_network("libera").expect("network");
    let (tx, mut rx) = tokio::sync::mpsc::channel(4);
    client
        .ingest(NetworkId(1), row, item(Some("m1"), "hello", 1_000), tx)
        .expect("send");
    let buffer = drain(&mut rx, 1)[0].buffer;

    assert!(
        buffers(&client)[0].last_read_seq.is_none(),
        "no marker until one is set"
    );
    set_marker(&client, buffer, Seq(1)).expect("marker set");
    assert_eq!(buffers(&client)[0].last_read_seq, Some(Seq(1)));

    drop(storage);
    let _ = std::fs::remove_dir_all(&dir);
}

/// **A marker may move backward, and last write wins.** The client is the
/// authority on where a person has read to; scrolling back to an unread point is
/// a real product action, and a monotonic clamp in the engine would refuse it
/// with no way to report the refusal. "Highest wins" is also the reconciliation
/// rule PLAN's Still-open owns — deciding it inside the UPDATE would answer that
/// question by accident.
#[test]
fn a_read_marker_may_move_backward() {
    let (dir, storage, client) = temp_store();
    let row = client.ensure_network("libera").expect("network");
    let (tx, mut rx) = tokio::sync::mpsc::channel(8);
    for i in 1..=5 {
        client
            .ingest(
                NetworkId(1),
                row,
                item(Some(&format!("m{i}")), "line", 1_000 + i),
                tx.clone(),
            )
            .expect("send");
    }
    let buffer = drain(&mut rx, 5)[0].buffer;

    set_marker(&client, buffer, Seq(5)).expect("forward");
    set_marker(&client, buffer, Seq(3)).expect("backward is legal");
    assert_eq!(
        buffers(&client)[0].last_read_seq,
        Some(Seq(3)),
        "last write wins, no clamp"
    );

    drop(storage);
    let _ = std::fs::remove_dir_all(&dir);
}

/// The two client bugs that must not be answered politely: marking nothing read,
/// and marking a buffer that does not exist. The seq is deliberately *not*
/// checked for existence — a marker is a position, not a row reference, and
/// retention will make gaps real.
#[test]
fn read_marker_rejects_zero_seq_and_unknown_buffer() {
    let (dir, storage, client) = temp_store();
    let row = client.ensure_network("libera").expect("network");
    let (tx, mut rx) = tokio::sync::mpsc::channel(4);
    client
        .ingest(NetworkId(1), row, item(Some("m1"), "hello", 1_000), tx)
        .expect("send");
    let buffer = drain(&mut rx, 1)[0].buffer;

    let zero = set_marker(&client, buffer, Seq(0)).expect_err("zero is a client bug");
    assert!(zero.contains("at least 1"), "{zero}");
    let unknown =
        set_marker(&client, BufferId(9_999), Seq(1)).expect_err("unknown buffer is an error");
    assert!(unknown.contains("unknown buffer 9999"), "{unknown}");
    // A seq far past the end is *not* an error: positions outlive rows.
    set_marker(&client, buffer, Seq(9_999)).expect("a position, not a row reference");

    drop(storage);
    let _ = std::fs::remove_dir_all(&dir);
}

fn buffers(client: &StorageClient) -> Vec<BufferRow> {
    let (tx, mut rx) = tokio::sync::mpsc::channel(4);
    client
        .list_buffers(crate::bus::ClientId(1), tx)
        .expect("send");
    match rx.blocking_recv().expect("outcome") {
        ReadOutcome::Buffers { result, .. } => result.expect("list ok"),
        other => panic!("expected a buffer list, got {other:?}"),
    }
}

fn set_marker(client: &StorageClient, buffer: BufferId, seq: Seq) -> Result<(), String> {
    let (tx, mut rx) = tokio::sync::mpsc::channel(4);
    client
        .set_read_marker(
            buffer,
            seq,
            crate::bus::ClientId(1),
            havoc_ipc::RequestId(1),
            tx,
        )
        .expect("send");
    match rx.blocking_recv().expect("outcome") {
        ReadOutcome::MarkerSet { result, .. } => result,
        other => panic!("expected a marker outcome, got {other:?}"),
    }
}
