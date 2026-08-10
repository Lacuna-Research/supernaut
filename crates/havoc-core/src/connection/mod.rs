//! The per-network connection state machine — the crown jewels (NORTH-STAR
//! §5.2) — built sans-I/O: server lines in, client lines out, no sockets, no
//! runtime. Protocol bugs and I/O bugs stay separable forever (prompts 5–6 add
//! the I/O around this).
//!
//! `irc-proto` is used for `Message` parse/serialize only; every decision about
//! *when* to say what lives here.

pub mod actor;
mod caps;
pub mod io;

use std::collections::{BTreeMap, BTreeSet, HashMap};

use havoc_ipc::{ConnectionPhase, NetworkId};

/// Everything the actor needs to register on one network. Credentials arrive
/// as opaque inputs; where they are stored is prompt 10's problem.
#[derive(Debug, Clone)]
pub struct Config {
    pub nick: String,
    pub username: String,
    pub realname: String,
    /// `None` disables SASL entirely. `Some` makes it required: failure or
    /// denial of the `sasl` cap aborts the connection (fail-closed — falling
    /// back to plaintext auth paths is exactly the trap, NORTH-STAR §2.3).
    pub sasl: Option<SaslConfig>,
    pub autojoin: Vec<String>,
}

/// Ordered mechanism preference list, per the 2026-08-10 SASL decision: stage 1
/// implements PLAIN only, but the *shape* is a list so EXTERNAL (CertFP) drops
/// in later without reshaping the machine.
#[derive(Debug, Clone)]
pub struct SaslConfig {
    pub mechanisms: Vec<SaslMechanism>,
    pub credentials: SaslCredentials,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SaslMechanism {
    Plain,
}

impl SaslMechanism {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Plain => "PLAIN",
        }
    }
}

#[derive(Clone)]
pub struct SaslCredentials {
    pub authcid: String,
    pub password: String,
}

/// Manual impl: the password must never print through a `{:?}` of `Machine`
/// or any actor log line — the secret prompt 10 stores must not leak here.
impl std::fmt::Debug for SaslCredentials {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SaslCredentials")
            .field("authcid", &self.authcid)
            .field("password", &"<redacted>")
            .finish()
    }
}

/// The machine's real states. The wire's `ConnectionPhase` is a deliberate
/// 3-variant projection of these (see [`Machine::phase`]) — this enum never
/// crosses the boundary (prompt-2 carry-forward).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum State {
    /// CAP LS sent, waiting for the (possibly multiline) capability list.
    CapLs,
    /// CAP REQ sent; waiting for every requested cap to resolve (ACK or NAK,
    /// any order) and for SASL if it is in flight.
    CapNegotiating,
    /// Caps resolved; AUTHENTICATE exchange in progress. CAP END is not sent
    /// until this completes (NORTH-STAR §6.8).
    SaslAuthenticating,
    /// CAP END sent; waiting for RPL_WELCOME.
    Registering,
    /// 001 received; collecting ISUPPORT, waiting for end-of-MOTD to autojoin.
    Registered,
    /// Autojoin dispatched; normal traffic.
    Steady,
    /// The reconnect seam, named but deliberately unimplemented: backoff is
    /// prompt 6, CHATHISTORY resync is stage 5. [`Machine::on_disconnect`] is
    /// the entry point the I/O layer will call.
    Disconnected,
    /// Fatal for this connection attempt (SASL failure/denial with SASL
    /// required). The actor disconnects; retry policy is prompt 6's.
    Failed { reason: String },
}

/// One per network, owned by its actor task. Instantiated from a
/// `HashMap<NetworkId, _>` from commit one (NORTH-STAR §6.9) even while
/// exactly one network exists — see [`Networks`].
#[derive(Debug)]
pub struct Machine {
    state: State,
    config: Config,
    /// Caps the server offered in LS (accumulated across continuation lines).
    offered: BTreeSet<String>,
    /// Caps requested and not yet ACKed/NAKed.
    pending: BTreeSet<String>,
    /// Caps currently enabled.
    enabled: BTreeSet<String>,
    /// LS continuation accumulator active.
    ls_in_progress: bool,
    sasl_outcome: Option<bool>,
    /// Nick as the server last confirmed it (001 may differ from requested).
    nick: String,
    isupport: BTreeMap<String, String>,
}

impl Machine {
    /// Create the machine and emit the opening client lines:
    /// `CAP LS 302`, `NICK`, `USER`.
    pub fn start(config: Config) -> (Self, Vec<String>) {
        let nick = config.nick.clone();
        let lines = vec![
            "CAP LS 302".to_owned(),
            format!("NICK {nick}"),
            format!("USER {} 0 * :{}", config.username, config.realname),
        ];
        (
            Self {
                state: State::CapLs,
                config,
                offered: BTreeSet::new(),
                pending: BTreeSet::new(),
                enabled: BTreeSet::new(),
                ls_in_progress: false,
                sasl_outcome: None,
                nick,
                isupport: BTreeMap::new(),
            },
            lines,
        )
    }

    pub fn state(&self) -> &State {
        &self.state
    }

    /// The boundary projection onto the wire's coarse enum. The real states
    /// stay core-private; widening `ConnectionPhase` is a wire change under
    /// `PROTOCOL_VERSION`.
    pub fn phase(&self) -> ConnectionPhase {
        match self.state {
            State::Registered | State::Steady => ConnectionPhase::Registered,
            State::Disconnected | State::Failed { .. } => ConnectionPhase::Disconnected,
            _ => ConnectionPhase::Connecting,
        }
    }

    pub fn enabled_caps(&self) -> &BTreeSet<String> {
        &self.enabled
    }

    pub fn isupport(&self) -> &BTreeMap<String, String> {
        &self.isupport
    }

    pub fn nick(&self) -> &str {
        &self.nick
    }

    /// The I/O layer calls this when the transport drops. The seam for
    /// prompt 6's backoff and stage 5's resync.
    pub fn on_disconnect(&mut self) {
        self.state = State::Disconnected;
    }

    /// Feed one raw server line; returns the client lines to send in response.
    /// Unparseable lines are ignored — real networks emit garbage and a
    /// logging decision belongs to the actor, not the protocol machine.
    pub fn handle_line(&mut self, line: &str) -> Vec<String> {
        let Ok(message) = line.parse::<irc_proto::Message>() else {
            return Vec::new();
        };
        self.handle_message(&message)
    }

    fn handle_message(&mut self, message: &irc_proto::Message) -> Vec<String> {
        use irc_proto::Command;

        match &message.command {
            // Liveness first: PING is answered in every state.
            Command::PING(token, _) => vec![format!("PONG :{token}")],
            Command::CAP(_, sub, a, b) => self.handle_cap(*sub, a.as_deref(), b.as_deref()),
            Command::AUTHENTICATE(payload) => self.handle_authenticate_prompt(payload),
            Command::Response(code, args) => self.handle_numeric(*code, args),
            _ => Vec::new(),
        }
    }

    fn handle_numeric(&mut self, code: irc_proto::Response, args: &[String]) -> Vec<String> {
        use irc_proto::Response as R;

        match code {
            R::RPL_WELCOME => {
                if let Some(confirmed) = args.first() {
                    confirmed.clone_into(&mut self.nick);
                }
                self.state = State::Registered;
                Vec::new()
            }
            R::RPL_ISUPPORT => {
                self.absorb_isupport(args);
                Vec::new()
            }
            R::RPL_ENDOFMOTD | R::ERR_NOMOTD => self.autojoin(),
            R::RPL_SASLSUCCESS => self.sasl_finished(true),
            R::ERR_SASLFAIL | R::ERR_SASLABORT | R::ERR_SASLTOOLONG => self.sasl_finished(false),
            _ => Vec::new(),
        }
    }

    /// `005` tokens: `KEY=VALUE` or bare `KEY`, between the nick argument and
    /// the trailing "are supported" text.
    fn absorb_isupport(&mut self, args: &[String]) {
        for token in args.iter().skip(1).rev().skip(1).rev() {
            match token.split_once('=') {
                Some((key, value)) => {
                    self.isupport.insert(key.to_owned(), value.to_owned());
                }
                None => {
                    self.isupport.insert(token.clone(), String::new());
                }
            }
        }
    }

    fn autojoin(&mut self) -> Vec<String> {
        if self.state != State::Registered {
            return Vec::new();
        }
        self.state = State::Steady;
        if self.config.autojoin.is_empty() {
            Vec::new()
        } else {
            vec![format!("JOIN {}", self.config.autojoin.join(","))]
        }
    }
}

/// The actor map, from commit one (NORTH-STAR §6.9): multi-network is a
/// `HashMap` entry, never a retrofit. Holds actor handles — each task owns
/// its `Machine` outright; no state is shared (§5.5).
#[derive(Debug, Default)]
pub struct Networks {
    actors: HashMap<NetworkId, actor::ActorHandle>,
}

impl Networks {
    pub fn insert(&mut self, id: NetworkId, handle: actor::ActorHandle) {
        self.actors.insert(id, handle);
    }

    pub fn get(&self, id: NetworkId) -> Option<&actor::ActorHandle> {
        self.actors.get(&id)
    }

    pub fn iter(&self) -> impl Iterator<Item = (&NetworkId, &actor::ActorHandle)> {
        self.actors.iter()
    }
}
