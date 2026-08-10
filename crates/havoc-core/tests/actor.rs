//! Actor-level tests: the reconnect loop, backoff, and the fail-closed
//! no-retry contract — I/O-layer behavior, driven through real sockets and
//! paused tokio time. The protocol corpus lives in state_machine.rs.

use havoc_core::connection::{Config, SaslConfig, SaslCredentials, SaslMechanism};
use havoc_ipc::{ConnectionPhase, NetworkId};

fn config(sasl: bool) -> Config {
    Config {
        nick: "hvc".to_owned(),
        username: "hvc".to_owned(),
        realname: "supernaut".to_owned(),
        sasl: sasl.then(|| SaslConfig {
            mechanisms: vec![SaslMechanism::Plain],
            credentials: SaslCredentials {
                authcid: "alice".to_owned(),
                password: "sesame".to_owned(),
            },
        }),
        autojoin: vec!["#supernaut".to_owned(), "#tools".to_owned()],
    }
}

/// Actor-level test replacing the old map-of-machines test: the `Networks`
/// map now holds actor handles (one map from commit one, §6.9), and an actor
/// pointed at a dead port reports Connecting then Disconnected with a detail —
/// the seam prompt 6's backoff policy will drive.
#[tokio::test]
async fn actor_map_reports_connecting_then_disconnected() {
    use havoc_core::connection::Networks;
    use havoc_core::connection::actor::{self, ActorReport, ActorSpawn};

    let (reports_tx, mut reports) = tokio::sync::mpsc::unbounded_channel();
    let mut networks = Networks::default();
    // Reserved port with nothing listening: connect must fail fast.
    let handle = actor::spawn(ActorSpawn {
        network: NetworkId(1),
        host: "127.0.0.1".to_owned(),
        port: 9,
        security: havoc_core::connection::io::Security::Plaintext,
        config: config(false),
        reports: reports_tx,
        trace: false,
    });
    networks.insert(NetworkId(1), handle);
    assert_eq!(networks.iter().count(), 1);

    let (id, first) = reports.recv().await.expect("connecting report");
    assert_eq!(id, NetworkId(1));
    assert!(matches!(
        first,
        ActorReport::Phase {
            phase: ConnectionPhase::Connecting,
            ..
        }
    ));
    let (_, second) = reports.recv().await.expect("disconnect report");
    match second {
        ActorReport::Phase {
            phase: ConnectionPhase::Disconnected,
            detail,
        } => assert!(detail.is_some(), "a failed connect must say why"),
        other => panic!("expected Disconnected, got {other:?}"),
    }
    networks
        .get(NetworkId(1))
        .expect("handle present")
        .task
        .abort();
}

/// Reconnect proof at the actor level, wall-clock-free: a refused connect
/// retries through backoff (Connecting, Disconnected, Connecting again).
#[tokio::test(start_paused = true)]
async fn refused_connect_retries_through_backoff() {
    use havoc_core::connection::actor::{self, ActorReport, ActorSpawn};
    use havoc_core::connection::io::Security;

    let (reports_tx, mut reports) = tokio::sync::mpsc::unbounded_channel();
    let handle = actor::spawn(ActorSpawn {
        network: NetworkId(1),
        host: "127.0.0.1".to_owned(),
        port: 9,
        security: Security::Plaintext,
        config: config(false),
        reports: reports_tx,
        trace: false,
    });

    let mut phases = Vec::new();
    while phases.len() < 3 {
        let (_, report) = reports.recv().await.expect("report");
        if let ActorReport::Phase { phase, detail } = report {
            phases.push((phase, detail));
        }
    }
    assert_eq!(phases[0].0, ConnectionPhase::Connecting);
    assert_eq!(phases[1].0, ConnectionPhase::Disconnected);
    assert_eq!(phases[2].0, ConnectionPhase::Connecting);
    assert!(
        phases[2].1.as_deref().is_some_and(|d| d.contains("retry")),
        "the retry attempt must say so: {:?}",
        phases[2].1
    );
    handle.task.abort();
}

/// Fail-closed is never retried: a server that denies the sasl cap while SASL
/// is configured yields exactly one Disconnected (with the reason) and the
/// actor task ends. The listener is a socket-level peer, deliberately not a
/// second protocol fake.
#[tokio::test(start_paused = true)]
async fn sasl_denial_is_reported_once_and_never_retried() {
    use havoc_core::connection::actor::{self, ActorReport, ActorSpawn};
    use havoc_core::connection::io::Security;
    use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};

    let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
        .await
        .expect("bind");
    let port = listener.local_addr().expect("addr").port();
    tokio::spawn(async move {
        let (stream, _) = listener.accept().await.expect("accept");
        let (read, mut write) = tokio::io::split(stream);
        let mut lines = BufReader::new(read).lines();
        write
            .write_all(
                b":s CAP * LS :sasl server-time
",
            )
            .await
            .expect("ls");
        while let Ok(Some(line)) = lines.next_line().await {
            if line.starts_with("CAP REQ") {
                write
                    .write_all(
                        b":s CAP hvc NAK :sasl
",
                    )
                    .await
                    .expect("nak");
            }
        }
    });

    let (reports_tx, mut reports) = tokio::sync::mpsc::unbounded_channel();
    let handle = actor::spawn(ActorSpawn {
        network: NetworkId(1),
        host: "127.0.0.1".to_owned(),
        port,
        security: Security::Plaintext,
        config: config(true),
        reports: reports_tx,
        trace: false,
    });

    let mut disconnects = Vec::new();
    while let Some((_, report)) = reports.recv().await {
        if let ActorReport::Phase {
            phase: ConnectionPhase::Disconnected,
            detail,
        } = report
        {
            disconnects.push(detail);
        }
    }
    // Channel closed = actor task ended. Exactly one Disconnected, with the
    // fail-closed reason; no retry ever happened.
    assert_eq!(disconnects.len(), 1, "Failed must never be retried");
    assert!(
        disconnects[0]
            .as_deref()
            .is_some_and(|d| d.contains("SASL")),
        "the reason must survive to the report: {disconnects:?}"
    );
    handle.task.await.expect("actor ended cleanly");
}
