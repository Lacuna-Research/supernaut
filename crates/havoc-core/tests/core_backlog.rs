//! Dispatch-level backlog test: attach over a store an earlier process wrote,
//! watch the buffers this session never saw created get *announced* to it on its
//! own lane (§4.5's attach contract, with no request added to §4.7's fenced
//! surface), then page a window out of history.

use std::collections::HashMap;
use std::time::Duration;

use havoc_core::bus::Directed;
use havoc_core::connection::io::Security;
use havoc_core::core::{Core, NetworkSettings};
use havoc_core::storage::{Storage, StorageClient};
use havoc_ipc::{
    Anchor, BufferId, Event, NetworkId, Request, RequestBody, RequestId, ResponseBody,
};

fn settings(name: &str) -> NetworkSettings {
    NetworkSettings {
        name: name.to_owned(),
        host: "127.0.0.1".to_owned(),
        port: 6667,
        security: Security::Plaintext,
        connection: havoc_core::connection::Config {
            nick: "alice".to_owned(),
            username: "alice".to_owned(),
            realname: "backlog test".to_owned(),
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

/// Five rows into `#seed` on network `seed-net`, through the real ingest lane —
/// this stands in for "an earlier process wrote this file".
async fn seed(client: &StorageClient) {
    let row = client.ensure_network("seed-net").expect("network");
    let (tx, mut rx) = tokio::sync::mpsc::channel(8);
    for i in 1..=5 {
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
    for _ in 0..5 {
        rx.recv().await.expect("outcome");
    }
}

#[tokio::test]
async fn attach_announces_unseen_buffers_and_serves_windows() {
    let (dir, storage) = store("corebacklog");
    seed(&storage.client()).await;

    let core = Core::spawn(
        storage.client(),
        HashMap::from([(NetworkId(1), settings("seed-net"))]),
        false,
    );
    let mut a = core.attach().await;

    let announced = match a.directed.recv().await.expect("the attach replay") {
        Directed::Event(Event::BufferCreated { buffer }) => buffer,
        other => panic!("expected the attach replay first, got {other:?}"),
    };
    assert_eq!(announced.name, "#seed");
    assert_eq!(
        announced.network,
        NetworkId(1),
        "the storage row's network name resolved back to the caller's id"
    );
    assert!(
        announced.last_read_seq.is_none(),
        "nothing writes last_read_seq until prompt 9b"
    );
    assert!(
        a.broadcast.try_recv().is_err(),
        "the replay is one session's business; broadcasting it would announce \
         another client's history"
    );

    // A second session gets its own replay, and A gets no second copy.
    let mut b = core.attach().await;
    match b.directed.recv().await.expect("B's own replay") {
        Directed::Event(Event::BufferCreated { buffer }) => assert_eq!(buffer.id, announced.id),
        other => panic!("expected B's own replay, got {other:?}"),
    }
    assert!(
        a.directed.try_recv().is_err(),
        "B's replay must not land on A's lane"
    );

    // And the window itself, over the buffer only the replay could have named.
    a.requests
        .send((
            a.id,
            Request {
                id: RequestId(9),
                body: RequestBody::FetchBacklog {
                    buffer: announced.id,
                    anchor: Anchor::Latest,
                    limit: 3,
                },
            },
        ))
        .await
        .expect("send");
    match a.directed.recv().await.expect("the window") {
        Directed::Response(response) => {
            assert_eq!(response.id, RequestId(9));
            match response.body {
                ResponseBody::Backlog { messages } => {
                    assert_eq!(messages.len(), 3);
                    assert_eq!(messages[0].text, "seed 3", "ascending, newest three");
                    assert_eq!(messages[2].text, "seed 5");
                }
                other => panic!("expected a Backlog response, got {other:?}"),
            }
        }
        other => panic!("expected the correlated Response, got {other:?}"),
    }

    drop(core);
    drop(storage);
    let _ = std::fs::remove_dir_all(&dir);
}

/// A buffer whose network is not configured is skipped: a `BufferInfo` carrying
/// a `NetworkId` the client cannot name is worse than an absence, and the
/// history reappears the moment the network returns to config.
#[tokio::test]
async fn a_buffer_on_an_unconfigured_network_is_not_announced() {
    let (dir, storage) = store("corebacklog-skip");
    seed(&storage.client()).await;

    let core = Core::spawn(
        storage.client(),
        HashMap::from([(NetworkId(1), settings("some-other-net"))]),
        false,
    );
    let mut a = core.attach().await;

    // Deterministic, not timed: the ListBuffers job is enqueued at attach and
    // the storage thread answers jobs in order, so if the first thing on the
    // lane is this Response, no announcement was queued ahead of it.
    a.requests
        .send((
            a.id,
            Request {
                id: RequestId(1),
                body: RequestBody::FetchBacklog {
                    buffer: BufferId(1),
                    anchor: Anchor::Latest,
                    limit: 1,
                },
            },
        ))
        .await
        .expect("send");
    match a.directed.recv().await.expect("the window") {
        Directed::Response(response) => {
            assert!(matches!(response.body, ResponseBody::Backlog { .. }));
        }
        other => panic!("expected no announcement, got {other:?}"),
    }
    // And none arrives late from the spawned delivery task either.
    assert!(
        tokio::time::timeout(Duration::from_millis(300), a.directed.recv())
            .await
            .is_err(),
        "an unresolvable buffer must never be announced"
    );

    drop(core);
    drop(storage);
    let _ = std::fs::remove_dir_all(&dir);
}
