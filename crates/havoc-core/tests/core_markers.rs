//! Dispatch-level read-marker test: the write answers `Ack` to the session that
//! asked *and* `ReadMarkerChanged` to every attached client on the **broadcast**
//! lane — the lane is the claim under test, because it is what makes the marker
//! core-owned machine state rather than one client's private note (§4.5's Core
//! column). A failed write is not state: Error response, no event.

use std::collections::HashMap;
use std::time::Duration;

use havoc_core::bus::Directed;
use havoc_core::connection::io::Security;
use havoc_core::core::{Core, NetworkSettings};
use havoc_core::storage::{Storage, StorageClient};
use havoc_ipc::{BufferId, Event, NetworkId, Request, RequestBody, RequestId, ResponseBody, Seq};

fn settings(name: &str) -> NetworkSettings {
    NetworkSettings {
        name: name.to_owned(),
        host: "127.0.0.1".to_owned(),
        port: 6667,
        security: Security::Plaintext,
        connection: havoc_core::connection::Config {
            nick: "alice".to_owned(),
            username: "alice".to_owned(),
            realname: "marker test".to_owned(),
            sasl: None,
            autojoin: Vec::new(),
        },
    }
}

fn store(tag: &str) -> (std::path::PathBuf, Storage) {
    let dir = std::env::temp_dir().join(format!("havoc-{tag}-{}", std::process::id()));
    std::fs::create_dir_all(&dir).expect("temp dir");
    let path = dir.join("h.db");
    let _ = std::fs::remove_file(&path);
    let (storage, _) = Storage::open(&path, false).expect("open");
    (dir, storage)
}

/// Three rows into `#seed` through the real ingest lane, standing in for history
/// an earlier process wrote.
async fn seed(client: &StorageClient) {
    let row = client.ensure_network("seed-net").expect("network");
    let (tx, mut rx) = tokio::sync::mpsc::channel(8);
    for i in 1..=3 {
        client
            .ingest(
                NetworkId(1),
                row,
                havoc_core::storage::Ingest {
                    target: "#seed".to_owned(),
                    kind: havoc_ipc::MessageKind::Privmsg,
                    nick: Some("alice".to_owned()),
                    account: None,
                    text: Some(format!("seed {i}")),
                    server_time: havoc_ipc::ServerTime::from_unix_millis(1_000 + i),
                    msgid: Some(format!("m{i}")),
                    tags: std::collections::BTreeMap::new(),
                },
                tx.clone(),
            )
            .expect("send");
    }
    for _ in 0..3 {
        rx.recv().await.expect("outcome");
    }
}

#[tokio::test]
async fn setting_a_marker_acks_the_requester_and_broadcasts_the_change() {
    let (dir, storage) = store("coremarker");
    seed(&storage.client()).await;

    let core = Core::spawn(
        storage.client(),
        HashMap::from([(NetworkId(1), settings("seed-net"))]),
        false,
    );
    let mut a = core.attach().await;
    let mut b = core.attach().await;

    let buffer = match a.directed.recv().await.expect("the attach replay") {
        Directed::Event(Event::BufferCreated { buffer }) => buffer,
        other => panic!("expected the attach replay first, got {other:?}"),
    };
    assert!(buffer.last_read_seq.is_none(), "no marker set yet");
    // Drain B's own replay so its lanes are quiet before the marker moves.
    let _ = b.directed.recv().await.expect("B's own replay");

    a.requests
        .send((
            a.id,
            Request {
                id: RequestId(11),
                body: RequestBody::SetReadMarker {
                    buffer: buffer.id,
                    seq: Seq(2),
                },
            },
        ))
        .await
        .expect("send");

    match a.directed.recv().await.expect("the Ack") {
        Directed::Response(response) => {
            assert_eq!(response.id, RequestId(11));
            assert!(
                matches!(response.body, ResponseBody::Ack),
                "the write's Ack"
            );
        }
        other => panic!("expected the Ack on the requester's lane, got {other:?}"),
    }
    // The claim: a marker one client moved is a marker moved for the machine, so
    // it arrives on the *broadcast* lane of a session that asked for nothing.
    match b.broadcast.recv().await.expect("the broadcast event") {
        Event::ReadMarkerChanged { buffer: id, seq } => {
            assert_eq!(id, buffer.id);
            assert_eq!(seq, Seq(2));
        }
        other => panic!("expected ReadMarkerChanged on B's broadcast lane, got {other:?}"),
    }
    assert!(
        b.directed.try_recv().is_err(),
        "B asked for nothing, so nothing correlated is owed to it"
    );

    drop(core);
    drop(storage);
    let _ = std::fs::remove_dir_all(&dir);
}

/// An unknown buffer is a client bug: the Error response goes back, and **no
/// event** is emitted — a failed write is not state.
#[tokio::test]
async fn an_unknown_buffer_errors_and_emits_no_event() {
    let (dir, storage) = store("coremarker-unknown");
    seed(&storage.client()).await;

    let core = Core::spawn(
        storage.client(),
        HashMap::from([(NetworkId(1), settings("seed-net"))]),
        false,
    );
    let mut a = core.attach().await;
    let mut b = core.attach().await;
    let _ = a.directed.recv().await.expect("A's replay");
    let _ = b.directed.recv().await.expect("B's replay");

    a.requests
        .send((
            a.id,
            Request {
                id: RequestId(12),
                body: RequestBody::SetReadMarker {
                    buffer: BufferId(9_999),
                    seq: Seq(1),
                },
            },
        ))
        .await
        .expect("send");

    match a.directed.recv().await.expect("the error") {
        Directed::Response(response) => {
            assert_eq!(response.id, RequestId(12));
            match response.body {
                ResponseBody::Error { message } => {
                    assert!(message.contains("unknown buffer 9999"), "{message}");
                }
                other => panic!("expected an Error response, got {other:?}"),
            }
        }
        other => panic!("expected the Error on the requester's lane, got {other:?}"),
    }
    assert!(
        tokio::time::timeout(Duration::from_millis(300), b.broadcast.recv())
            .await
            .is_err(),
        "a failed write must announce nothing"
    );

    drop(core);
    drop(storage);
    let _ = std::fs::remove_dir_all(&dir);
}
