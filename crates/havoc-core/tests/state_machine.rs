//! Table-driven transcript tests for the connection state machine (NORTH-STAR
//! §6.8): server lines in → expected client lines out, plus final-state
//! assertions. Hand-written from real-server behavior (Libera/ergo shapes);
//! prompt 6 captures live transcripts into this same corpus.

use havoc_core::connection::{Config, Machine, SaslConfig, SaslCredentials, SaslMechanism, State};
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

/// One transcript step: a server line and every client line it must produce
/// (in order). The empty list asserts silence.
struct Step(&'static str, &'static [&'static str]);

fn run(machine: &mut Machine, steps: &[Step]) {
    for (index, Step(server_line, expected)) in steps.iter().enumerate() {
        let out = machine.handle_line(server_line);
        assert_eq!(
            out,
            *expected,
            "step {index}: server line {server_line:?} (state {:?})",
            machine.state()
        );
    }
}

#[test]
fn happy_path_with_sasl_multiline_ls_and_autojoin() {
    let (mut m, opening) = Machine::start(config(true));
    assert_eq!(
        opening,
        ["CAP LS 302", "NICK hvc", "USER hvc 0 * :supernaut"]
    );

    run(
        &mut m,
        &[
            // Multiline LS: continuation marked by "*".
            Step(
                ":irc.example CAP * LS * :server-time message-tags batch",
                &[],
            ),
            Step(
                ":irc.example CAP * LS :echo-message labeled-response sasl=PLAIN,EXTERNAL",
                &["CAP REQ :server-time message-tags echo-message batch labeled-response sasl"],
            ),
            // ACK everything at once; SASL starts, CAP END is withheld.
            Step(
                ":irc.example CAP hvc ACK :server-time message-tags echo-message batch labeled-response sasl",
                &["AUTHENTICATE PLAIN"],
            ),
            Step("AUTHENTICATE +", &["AUTHENTICATE AGFsaWNlAHNlc2FtZQ=="]),
            Step(
                ":irc.example 900 hvc hvc!u@h alice :You are now logged in as alice",
                &[],
            ),
            // Only 903 releases CAP END.
            Step(
                ":irc.example 903 hvc :SASL authentication successful",
                &["CAP END"],
            ),
            Step(
                ":irc.example 001 hvc :Welcome to the Example IRC Network hvc",
                &[],
            ),
            Step(
                ":irc.example 005 hvc NETWORK=Example CHANTYPES=# MONITOR=100 :are supported by this server",
                &[],
            ),
            // End of MOTD triggers autojoin.
            Step(
                ":irc.example 376 hvc :End of /MOTD command.",
                &["JOIN #supernaut,#tools"],
            ),
            // Liveness in steady state.
            Step("PING :token-1", &["PONG :token-1"]),
        ],
    );

    assert_eq!(*m.state(), State::Steady);
    assert_eq!(m.phase(), ConnectionPhase::Registered);
    assert!(m.enabled_caps().contains("sasl"));
    assert_eq!(
        m.isupport().get("NETWORK").map(String::as_str),
        Some("Example")
    );
    assert_eq!(m.isupport().get("CHANTYPES").map(String::as_str), Some("#"));
    assert_eq!(m.nick(), "hvc");
}

#[test]
fn non_sasl_cap_denial_is_tolerated() {
    let (mut m, _) = Machine::start(config(true));
    run(
        &mut m,
        &[
            Step(
                ":s CAP * LS :server-time message-tags echo-message batch labeled-response sasl",
                &["CAP REQ :server-time message-tags echo-message batch labeled-response sasl"],
            ),
            // Denials of ordinary caps: registration proceeds without them.
            Step(":s CAP hvc NAK :echo-message labeled-response", &[]),
            Step(
                ":s CAP hvc ACK :server-time message-tags batch sasl",
                &["AUTHENTICATE PLAIN"],
            ),
            Step("AUTHENTICATE +", &["AUTHENTICATE AGFsaWNlAHNlc2FtZQ=="]),
            Step(":s 903 hvc :ok", &["CAP END"]),
        ],
    );
    assert_eq!(*m.state(), State::Registering);
    assert!(!m.enabled_caps().contains("echo-message"));
    assert!(m.enabled_caps().contains("server-time"));
}

#[test]
fn sasl_failure_is_fatal_when_sasl_is_configured() {
    let (mut m, _) = Machine::start(config(true));
    run(
        &mut m,
        &[
            Step(
                ":s CAP * LS :sasl server-time",
                &["CAP REQ :server-time sasl"],
            ),
            Step(":s CAP hvc ACK :server-time sasl", &["AUTHENTICATE PLAIN"]),
            Step("AUTHENTICATE +", &["AUTHENTICATE AGFsaWNlAHNlc2FtZQ=="]),
            // 904: no CAP END, no fallback — fail-closed.
            Step(":s 904 hvc :SASL authentication failed", &[]),
        ],
    );
    assert!(matches!(m.state(), State::Failed { .. }));
    assert_eq!(m.phase(), ConnectionPhase::Disconnected);
}

#[test]
fn sasl_cap_denial_is_fatal_when_sasl_is_configured() {
    let (mut m, _) = Machine::start(config(true));
    run(
        &mut m,
        &[
            Step(
                ":s CAP * LS :sasl server-time",
                &["CAP REQ :server-time sasl"],
            ),
            Step(":s CAP hvc NAK :sasl", &[]),
        ],
    );
    assert!(matches!(m.state(), State::Failed { .. }));
}

#[test]
fn sasl_required_but_not_offered_is_fatal() {
    let (mut m, _) = Machine::start(config(true));
    run(
        &mut m,
        &[Step(":s CAP * LS :server-time message-tags", &[])],
    );
    assert!(matches!(m.state(), State::Failed { .. }));
}

#[test]
fn out_of_order_ack_resolution_delays_cap_end_to_the_last_cap() {
    let (mut m, _) = Machine::start(config(false));
    run(
        &mut m,
        &[
            Step(
                ":s CAP * LS :server-time message-tags echo-message batch labeled-response",
                &["CAP REQ :server-time message-tags echo-message batch labeled-response"],
            ),
            // ACKs split across lines, out of request order; END only at the last.
            Step(":s CAP hvc ACK :batch echo-message", &[]),
            Step(":s CAP hvc ACK :labeled-response server-time", &[]),
            Step(":s CAP hvc ACK :message-tags", &["CAP END"]),
            // And the connection still ends registered, as the acceptance says.
            Step(":s 001 hvc :welcome", &[]),
            Step(":s 376 hvc :end", &["JOIN #supernaut,#tools"]),
        ],
    );
    assert_eq!(*m.state(), State::Steady);
    assert_eq!(m.phase(), ConnectionPhase::Registered);
}

/// Reviewer catch: a CAP NEW arriving mid-negotiation must gate CAP END until
/// its request resolves — "every requested cap" has no timing exception.
#[test]
fn cap_new_during_negotiation_gates_cap_end() {
    let (mut m, _) = Machine::start(config(false));
    run(
        &mut m,
        &[
            Step(
                ":s CAP * LS :server-time message-tags",
                &["CAP REQ :server-time message-tags"],
            ),
            Step(":s CAP hvc ACK :server-time", &[]),
            // Mid-negotiation NEW: batch gets requested and now also gates END.
            Step(":s CAP hvc NEW :batch", &["CAP REQ :batch"]),
            Step(":s CAP hvc ACK :message-tags", &[]),
            Step(":s CAP hvc ACK :batch", &["CAP END"]),
        ],
    );
    assert_eq!(*m.state(), State::Registering);
    assert!(m.enabled_caps().contains("batch"));
}

#[test]
fn no_sasl_configured_never_requests_sasl() {
    let (mut m, _) = Machine::start(config(false));
    run(
        &mut m,
        &[
            Step(":s CAP * LS :server-time sasl", &["CAP REQ :server-time"]),
            Step(":s CAP hvc ACK :server-time", &["CAP END"]),
            Step(":s 001 hvc :welcome", &[]),
            Step(
                ":s 422 hvc :MOTD File is missing",
                &["JOIN #supernaut,#tools"],
            ),
        ],
    );
    assert_eq!(*m.state(), State::Steady);
}

#[test]
fn cap_new_and_del_mid_session() {
    let (mut m, _) = Machine::start(config(false));
    run(
        &mut m,
        &[
            Step(":s CAP * LS :server-time", &["CAP REQ :server-time"]),
            Step(":s CAP hvc ACK :server-time", &["CAP END"]),
            Step(":s 001 hvc :welcome", &[]),
            Step(":s 376 hvc :end", &["JOIN #supernaut,#tools"]),
            // Mid-session NEW: the wanted subset is re-requested; unwanted isn't.
            Step(":s CAP hvc NEW :batch dcc-something", &["CAP REQ :batch"]),
            Step(":s CAP hvc ACK :batch", &[]),
            // Mid-session DEL revokes.
            Step(":s CAP hvc DEL :server-time", &[]),
        ],
    );
    assert_eq!(
        *m.state(),
        State::Steady,
        "no CAP END may fire post-registration"
    );
    assert!(m.enabled_caps().contains("batch"));
    assert!(!m.enabled_caps().contains("server-time"));
}

#[test]
fn welcome_confirms_a_server_modified_nick() {
    let (mut m, _) = Machine::start(config(false));
    run(
        &mut m,
        &[
            Step(":s CAP * LS :server-time", &["CAP REQ :server-time"]),
            Step(":s CAP hvc ACK :server-time", &["CAP END"]),
            Step(":s 001 hvc_ :welcome", &[]),
        ],
    );
    assert_eq!(m.nick(), "hvc_");
}

/// Actor-level test replacing the old map-of-machines test: the `Networks`
/// map now holds actor handles (one map from commit one, §6.9), and an actor
/// pointed at a dead port reports Connecting then Disconnected with a detail —
/// the seam prompt 6's backoff policy will drive.
#[tokio::test]
async fn actor_map_reports_connecting_then_disconnected() {
    use havoc_core::connection::Networks;
    use havoc_core::connection::actor::{self, ActorReport, ActorSpawn};

    let (reports_tx, mut reports) = tokio::sync::mpsc::channel(16);
    let mut networks = Networks::default();
    // Reserved port with nothing listening: connect must fail fast.
    let handle = actor::spawn(ActorSpawn {
        network: NetworkId(1),
        host: "127.0.0.1".to_owned(),
        port: 9,
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
