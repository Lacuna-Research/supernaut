//! IRCv3 capability negotiation and SASL — the part most likely to be subtly
//! wrong (NORTH-STAR §6.8). The rules, stated once:
//!
//! - SASL completes before CAP END.
//! - CAP END is sent only after every requested cap has resolved (ACK or NAK,
//!   in any order).
//! - CAP NEW / CAP DEL are handled mid-session.
//! - Any non-sasl cap may be denied and registration proceeds; sasl denial
//!   with SASL configured is fatal (fail-closed).

use irc_proto::CapSubCommand;

use super::{Machine, State};

/// Caps we request when offered. `sasl` is appended only when configured.
const WANTED: &[&str] = &[
    "server-time",
    "message-tags",
    "echo-message",
    "batch",
    "labeled-response",
];

impl Machine {
    pub(super) fn handle_cap(
        &mut self,
        sub: CapSubCommand,
        a: Option<&str>,
        b: Option<&str>,
    ) -> Vec<String> {
        // Server CAP lines put the cap list in the last present argument; a
        // literal "*" in the middle marks an LS/LIST continuation line.
        let (continuation, caps_arg) = match (a, b) {
            (Some("*"), Some(caps)) => (true, caps),
            (_, Some(caps)) => (false, caps),
            (Some(caps), None) => (false, caps),
            (None, None) => (false, ""),
        };

        match sub {
            CapSubCommand::LS => self.handle_ls(continuation, caps_arg),
            CapSubCommand::ACK => self.handle_ack(caps_arg),
            CapSubCommand::NAK => self.handle_nak(caps_arg),
            CapSubCommand::NEW => self.handle_new(caps_arg),
            CapSubCommand::DEL => self.handle_del(caps_arg),
            _ => Vec::new(),
        }
    }

    fn handle_ls(&mut self, continuation: bool, caps: &str) -> Vec<String> {
        // `cap` or `cap=value`; values (e.g. sasl=PLAIN,EXTERNAL) are recorded
        // with the offer.
        for cap in caps.split_whitespace() {
            self.offered.insert(cap.to_owned());
        }
        if continuation {
            self.ls_in_progress = true;
            return Vec::new();
        }
        self.ls_in_progress = false;
        if self.state != State::CapLs {
            // LS outside negotiation (e.g. a later `CAP LS` reply): ignore.
            return Vec::new();
        }

        let mut request: Vec<&str> = WANTED
            .iter()
            .copied()
            .filter(|cap| self.offers(cap))
            .collect();
        if self.config.sasl.is_some() {
            if self.offers("sasl") {
                request.push("sasl");
            } else {
                // Fail-closed: the server cannot authenticate us the way the
                // user demanded. Plaintext fallback is the trap (§2.3).
                self.state = State::Failed {
                    reason: "SASL required but not offered by server".to_owned(),
                };
                return Vec::new();
            }
        }

        if request.is_empty() {
            self.state = State::Registering;
            return vec!["CAP END".to_owned()];
        }
        self.pending = request.iter().map(|s| (*s).to_owned()).collect();
        self.state = State::CapNegotiating;
        vec![format!("CAP REQ :{}", request.join(" "))]
    }

    fn handle_ack(&mut self, caps: &str) -> Vec<String> {
        let mut out = Vec::new();
        for cap in caps.split_whitespace() {
            self.pending.remove(cap);
            self.enabled.insert(cap.to_owned());
            if cap == "sasl" && self.state == State::CapNegotiating {
                out.extend(self.begin_sasl());
            }
        }
        out.extend(self.maybe_cap_end());
        out
    }

    fn handle_nak(&mut self, caps: &str) -> Vec<String> {
        for cap in caps.split_whitespace() {
            self.pending.remove(cap);
            if cap == "sasl" && self.config.sasl.is_some() {
                self.state = State::Failed {
                    reason: "SASL required but the sasl cap was denied".to_owned(),
                };
                return Vec::new();
            }
        }
        self.maybe_cap_end()
    }

    /// New offers: request the wanted subset. ACKs arrive through the same
    /// handler. A NEW arriving *during* negotiation must gate CAP END like any
    /// other request — "after every requested cap has resolved" has no
    /// mid-negotiation exception (this was a reviewer catch, not a freebie).
    fn handle_new(&mut self, caps: &str) -> Vec<String> {
        let newly_wanted: Vec<&str> = caps
            .split_whitespace()
            .map(|cap| cap.split('=').next().unwrap_or(cap))
            .filter(|cap| WANTED.contains(cap) && !self.enabled.contains(*cap))
            .collect();
        for cap in caps.split_whitespace() {
            self.offered.insert(cap.to_owned());
        }
        if newly_wanted.is_empty() {
            return Vec::new();
        }
        let negotiating = matches!(
            self.state,
            State::CapNegotiating | State::SaslAuthenticating
        );
        if negotiating {
            for cap in &newly_wanted {
                self.pending.insert((*cap).to_owned());
            }
        }
        vec![format!("CAP REQ :{}", newly_wanted.join(" "))]
    }

    fn handle_del(&mut self, caps: &str) -> Vec<String> {
        for cap in caps.split_whitespace() {
            self.enabled.remove(cap);
            self.offered
                .retain(|offer| offer != cap && offer.split('=').next() != Some(cap));
        }
        Vec::new()
    }

    fn offers(&self, wanted: &str) -> bool {
        self.offered
            .iter()
            .any(|offer| offer == wanted || offer.split('=').next() == Some(wanted))
    }

    /// CAP END exactly when nothing is pending and SASL is not in flight
    /// (NORTH-STAR §6.8: after everything requested has resolved).
    fn maybe_cap_end(&mut self) -> Vec<String> {
        let sasl_in_flight = self.state == State::SaslAuthenticating && self.sasl_outcome.is_none();
        if self.state == State::Steady || self.state == State::Registered {
            return Vec::new();
        }
        if self.pending.is_empty() && !sasl_in_flight {
            self.state = State::Registering;
            vec!["CAP END".to_owned()]
        } else {
            Vec::new()
        }
    }

    fn begin_sasl(&mut self) -> Vec<String> {
        let Some(sasl) = &self.config.sasl else {
            return Vec::new();
        };
        // Ordered preference list; PLAIN is the only stage-1 entry, EXTERNAL
        // slots in here later (2026-08-10 decision).
        let Some(mechanism) = sasl.mechanisms.first() else {
            self.state = State::Failed {
                reason: "SASL configured with no mechanisms".to_owned(),
            };
            return Vec::new();
        };
        self.state = State::SaslAuthenticating;
        vec![format!("AUTHENTICATE {}", mechanism.as_str())]
    }

    /// Server's `AUTHENTICATE +` (or continuation data) during the exchange.
    pub(super) fn handle_authenticate_prompt(&mut self, payload: &str) -> Vec<String> {
        if self.state != State::SaslAuthenticating || payload != "+" {
            return Vec::new();
        }
        let Some(sasl) = &self.config.sasl else {
            return Vec::new();
        };
        // PLAIN: authzid \0 authcid \0 password, base64. Payloads beyond 400
        // bytes would need chunking; credentials that long are rejected by
        // servers anyway and EXTERNAL sends only "+".
        let raw = format!(
            "\0{}\0{}",
            sasl.credentials.authcid, sasl.credentials.password
        );
        vec![format!("AUTHENTICATE {}", base64(raw.as_bytes()))]
    }

    pub(super) fn sasl_finished(&mut self, success: bool) -> Vec<String> {
        if self.state != State::SaslAuthenticating {
            return Vec::new();
        }
        self.sasl_outcome = Some(success);
        if success {
            self.maybe_cap_end()
        } else {
            self.state = State::Failed {
                reason: "SASL authentication failed".to_owned(),
            };
            Vec::new()
        }
    }
}

/// Standard base64 (RFC 4648, with padding), encode only — hand-rolled rather
/// than a dependency: PLAIN needs sixteen lines of it and nothing else.
fn base64(input: &[u8]) -> String {
    const ALPHABET: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut out = String::with_capacity(input.len().div_ceil(3) * 4);
    for chunk in input.chunks(3) {
        let b = [
            chunk[0],
            *chunk.get(1).unwrap_or(&0),
            *chunk.get(2).unwrap_or(&0),
        ];
        let n = (u32::from(b[0]) << 16) | (u32::from(b[1]) << 8) | u32::from(b[2]);
        out.push(ALPHABET[(n >> 18) as usize & 63] as char);
        out.push(ALPHABET[(n >> 12) as usize & 63] as char);
        out.push(if chunk.len() > 1 {
            ALPHABET[(n >> 6) as usize & 63] as char
        } else {
            '='
        });
        out.push(if chunk.len() > 2 {
            ALPHABET[n as usize & 63] as char
        } else {
            '='
        });
    }
    out
}

#[cfg(test)]
mod tests {
    #[test]
    fn base64_matches_rfc_vectors() {
        assert_eq!(super::base64(b""), "");
        assert_eq!(super::base64(b"f"), "Zg==");
        assert_eq!(super::base64(b"fo"), "Zm8=");
        assert_eq!(super::base64(b"foo"), "Zm9v");
        assert_eq!(super::base64(b"foob"), "Zm9vYg==");
        assert_eq!(super::base64(b"fooba"), "Zm9vYmE=");
        assert_eq!(super::base64(b"foobar"), "Zm9vYmFy");
        // The exact PLAIN payload shape: \0authcid\0password.
        assert_eq!(super::base64(b"\0alice\0sesame"), "AGFsaWNlAHNlc2FtZQ==");
    }
}
