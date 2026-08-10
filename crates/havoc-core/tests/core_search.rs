//! Dispatch-level search test: A's search yields Response-then-SearchResults
//! in that order on A's directed lane while B's lanes stay silent — the leak
//! test the bus-level test cannot see, because routing happens in the core
//! select loop.

use std::collections::HashMap;

use havoc_core::bus::Directed;
use havoc_core::core::Core;
use havoc_core::storage::Storage;
use havoc_ipc::{NetworkId, Request, RequestBody, RequestId, ResponseBody};

#[tokio::test]
async fn search_results_reach_only_the_requester_in_order() {
    let dir = std::env::temp_dir().join(format!("havoc-coresearch-{}", std::process::id()));
    std::fs::create_dir_all(&dir).expect("temp dir");
    let path = dir.join("s.db");
    let _ = std::fs::remove_file(&path);
    let (storage, _) = Storage::open(&path, false).expect("open");

    // Seed one searchable row through the real ingest lane.
    let client = storage.client();
    let row = client.ensure_network("seed").expect("network");
    let (tx, mut rx) = tokio::sync::mpsc::channel(4);
    client
        .ingest(
            NetworkId(1),
            row,
            havoc_core::storage::Ingest {
                target: "#seed".to_owned(),
                kind: havoc_ipc::MessageKind::Privmsg,
                nick: Some("alice".to_owned()),
                account: None,
                text: Some("the deployment failed".to_owned()),
                server_time: havoc_ipc::ServerTime::from_unix_millis(1_000),
                msgid: Some("m1".to_owned()),
                tags: std::collections::BTreeMap::new(),
            },
            tx,
        )
        .expect("send");
    rx.recv().await.expect("outcome");

    let core = Core::spawn(storage.client(), HashMap::new(), false);
    let mut session_a = core.attach().await;
    let mut session_b = core.attach().await;

    session_a
        .requests
        .send((
            session_a.id,
            Request {
                id: RequestId(7),
                body: RequestBody::Search {
                    query: "deployment".to_owned(),
                },
            },
        ))
        .await
        .expect("send");

    // A: Response (Ack) first, then the correlated SearchResults.
    let first = session_a.directed.recv().await.expect("response");
    match first {
        Directed::Response(response) => {
            assert_eq!(response.id, RequestId(7));
            assert!(matches!(response.body, ResponseBody::Ack));
        }
        other => panic!("expected the Response first, got {other:?}"),
    }
    let second = session_a.directed.recv().await.expect("event");
    match second {
        Directed::Event(havoc_ipc::Event::SearchResults { request, hits }) => {
            assert_eq!(request, RequestId(7));
            assert_eq!(hits.len(), 1);
            assert_eq!(hits[0].text, "the deployment failed");
        }
        other => panic!("expected SearchResults second, got {other:?}"),
    }

    // B: nothing, on either lane.
    assert!(
        session_b.directed.try_recv().is_err(),
        "B's directed lane must stay silent"
    );
    assert!(
        session_b.broadcast.try_recv().is_err(),
        "B's broadcast lane must stay silent"
    );

    // A malformed MATCH string comes back as an Error response, no event.
    session_a
        .requests
        .send((
            session_a.id,
            Request {
                id: RequestId(8),
                body: RequestBody::Search {
                    query: "\"".to_owned(),
                },
            },
        ))
        .await
        .expect("send");
    let reply = session_a.directed.recv().await.expect("error response");
    match reply {
        Directed::Response(response) => {
            assert_eq!(response.id, RequestId(8));
            assert!(matches!(response.body, ResponseBody::Error { .. }));
        }
        other => panic!("expected an Error response, got {other:?}"),
    }

    drop(core);
    drop(storage);
    let _ = std::fs::remove_dir_all(&dir);
}
