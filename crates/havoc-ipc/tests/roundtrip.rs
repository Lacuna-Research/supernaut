//! Every wire type must survive CBOR — and the schema-evolution story
//! (NORTH-STAR §5.3) is proved here before there is a schema to evolve:
//! unknown fields in a struct are tolerated, which is what lets an upgraded
//! daemon talk to an older client across a capability intersection.

use std::collections::BTreeMap;

use ciborium::Value;
use havoc_ipc::{
    Anchor, BufferId, BufferInfo, BufferKind, ConnectionPhase, Event, Message, MessageKind,
    NetworkId, NetworkInfo, Request, RequestBody, RequestId, Response, ResponseBody, Seq,
    ServerTime,
};
use serde::{Serialize, de::DeserializeOwned};

fn roundtrip<T: Serialize + DeserializeOwned + PartialEq + std::fmt::Debug>(value: &T) {
    let mut bytes = Vec::new();
    ciborium::into_writer(value, &mut bytes).expect("serialize");
    let back: T = ciborium::from_reader(bytes.as_slice()).expect("deserialize");
    assert_eq!(&back, value);
}

fn sample_message() -> Message {
    let mut tags = BTreeMap::new();
    tags.insert("account".to_owned(), "alice".to_owned());
    Message {
        buffer: BufferId(7),
        seq: Seq(42),
        kind: MessageKind::Privmsg,
        nick: Some("alice".to_owned()),
        text: "the deployment failed".to_owned(),
        server_time: ServerTime::from_unix_millis(1_754_700_000_000),
        tags,
    }
}

#[test]
fn requests_roundtrip() {
    for body in [
        RequestBody::Connect {
            network: NetworkId(1),
        },
        RequestBody::Join {
            network: NetworkId(1),
            channel: "#supernaut".to_owned(),
        },
        RequestBody::SendText {
            buffer: BufferId(7),
            text: "hello".to_owned(),
        },
        RequestBody::FetchBacklog {
            buffer: BufferId(7),
            anchor: Anchor::Before(Seq(42)),
            limit: 200,
        },
        RequestBody::Search {
            query: "from:alice in:#supernaut deployment".to_owned(),
        },
        RequestBody::SetReadMarker {
            buffer: BufferId(7),
            seq: Seq(42),
        },
    ] {
        roundtrip(&Request {
            id: RequestId(9),
            body,
        });
    }
}

#[test]
fn responses_roundtrip() {
    for body in [
        ResponseBody::Ack,
        ResponseBody::Error {
            message: "no such buffer".to_owned(),
        },
        ResponseBody::Backlog {
            messages: vec![sample_message()],
        },
    ] {
        roundtrip(&Response {
            id: RequestId(9),
            body,
        });
    }
}

#[test]
fn events_roundtrip() {
    for event in [
        Event::ConnectionState {
            network: NetworkId(1),
            phase: ConnectionPhase::Registered,
            detail: None,
        },
        Event::ConnectionState {
            network: NetworkId(1),
            phase: ConnectionPhase::Disconnected,
            detail: Some("SASL authentication failed".to_owned()),
        },
        Event::BufferCreated {
            buffer: BufferInfo {
                id: BufferId(7),
                network: NetworkId(1),
                name: "#supernaut".to_owned(),
                kind: BufferKind::Channel,
                last_read_seq: Some(Seq(41)),
            },
        },
        Event::MessageAdded {
            message: sample_message(),
        },
        Event::SearchResults {
            request: RequestId(9),
            hits: vec![sample_message()],
        },
        Event::ReadMarkerChanged {
            buffer: BufferId(7),
            seq: Seq(42),
        },
    ] {
        roundtrip(&event);
    }
}

#[test]
fn all_anchors_roundtrip() {
    for anchor in [
        Anchor::Before(Seq(1)),
        Anchor::After(Seq(1)),
        Anchor::Latest,
        Anchor::AroundSearchHit(Seq(1)),
    ] {
        roundtrip(&anchor);
    }
}

#[test]
fn plain_models_roundtrip() {
    roundtrip(&NetworkInfo {
        id: NetworkId(1),
        name: "libera".to_owned(),
    });
    roundtrip(&sample_message());
}

/// A struct decoded from a CBOR map carrying a field this version has never
/// heard of must still decode — serde ignores unknown struct fields unless
/// told otherwise, and no type in havoc-ipc is ever told otherwise.
#[test]
fn unknown_struct_fields_are_tolerated() {
    let info = NetworkInfo {
        id: NetworkId(1),
        name: "libera".to_owned(),
    };
    let mut bytes = Vec::new();
    ciborium::into_writer(&info, &mut bytes).expect("serialize");

    let mut value: Value = ciborium::from_reader(bytes.as_slice()).expect("to value");
    let Value::Map(entries) = &mut value else {
        panic!("struct did not encode as a CBOR map");
    };
    entries.push((
        Value::Text("field_from_the_future".to_owned()),
        Value::Text("tolerated".to_owned()),
    ));

    let mut extended = Vec::new();
    ciborium::into_writer(&value, &mut extended).expect("re-serialize");
    let back: NetworkInfo = ciborium::from_reader(extended.as_slice()).expect("deserialize");
    assert_eq!(back, info);
}
