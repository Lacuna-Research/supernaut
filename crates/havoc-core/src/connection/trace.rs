//! Redaction for the `--trace-irc` transcript.
//!
//! Its own module, and not a corner of `actor.rs`, for two reasons: the rule is
//! about what a *log* may contain rather than about driving a connection, and
//! `actor.rs` sat one line under the `longest-file` ratchet's ceiling with this
//! helper in it — a stage boundary is the wrong place to leave the next prompt an
//! unbudgeted split.

use super::SaslMechanism;

/// The outbound trace line, with the SASL payload taken out.
///
/// `--trace-irc` writes every line we send to stderr, and scripts/live-run.sh
/// keeps a copy of that capture on disk — so without this the base64
/// `AUTHENTICATE <payload>` line *is* the password in a log file, which is the
/// one thing CLAUDE.md forbids outright. Base64 is not redaction.
///
/// Applied **at the trace only**, never at `send_line`: the wire must still carry
/// the real payload, and tests/state_machine.rs asserts that it does.
///
/// `AUTHENTICATE PLAIN` (the mechanism offer) and `AUTHENTICATE +` (the empty
/// response) carry nothing secret and pass through, so a trace still shows the
/// exchange's shape and live-run can still assert on it. A mechanism added to
/// [`SaslMechanism`] later would be over-redacted rather than leaked, which is
/// the direction to fail in.
pub(super) fn redact_outbound(line: &str) -> &str {
    let Some(argument) = line.strip_prefix("AUTHENTICATE ") else {
        return line;
    };
    if argument == "+" || argument == SaslMechanism::Plain.as_str() {
        return line;
    }
    "AUTHENTICATE <redacted>"
}

#[cfg(test)]
mod tests {
    /// The trace must show the SASL exchange's shape and none of its secret.
    #[test]
    fn the_outbound_trace_redacts_only_the_sasl_payload() {
        // The payload — base64 of `alice\0alice\0hunter2` — is the secret itself.
        assert_eq!(
            super::redact_outbound("AUTHENTICATE YWxpY2UAYWxpY2UAaHVudGVyMg=="),
            "AUTHENTICATE <redacted>"
        );
        // The mechanism offer and the empty response carry nothing.
        assert_eq!(
            super::redact_outbound("AUTHENTICATE PLAIN"),
            "AUTHENTICATE PLAIN"
        );
        assert_eq!(super::redact_outbound("AUTHENTICATE +"), "AUTHENTICATE +");
        // Everything else passes through untouched, including a message whose
        // text merely mentions the command.
        assert_eq!(
            super::redact_outbound("PRIVMSG #chan :AUTHENTICATE this"),
            "PRIVMSG #chan :AUTHENTICATE this"
        );
        assert_eq!(super::redact_outbound("CAP LS 302"), "CAP LS 302");
    }
}
