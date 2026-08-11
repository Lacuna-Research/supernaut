//! The config file's rules, all of them, on the public surface — which is why
//! this is an integration test rather than a `mod tests` inside config.rs: every
//! rule a hand-editing user can trip is reachable from outside the crate, and
//! keeping them here keeps config.rs well under the 400-line ratchet.
//!
//! Nothing here writes a config file to disk, and nothing can: `config::parse`
//! takes text and a base directory, and the `toml` serializer is not compiled
//! into this crate at all.

use std::path::{Path, PathBuf};

use havoc_core::config;
use havoc_core::connection::io::Security;
use havoc_ipc::NetworkId;

// `r##` because `["#supernaut"` contains the `"#` that would end an `r#` string.
const FULL: &str = r##"
# Seed data. Supernaut never writes this file.
nick = "alice"

[networks.libera]
host = "irc.libera.chat"
autojoin = ["#supernaut", "#rust"]
sasl_account = "alice"

[networks.ergo-local]
host = "localhost"
port = 6667
plaintext = true
"##;

fn base() -> PathBuf {
    PathBuf::from("/etc/supernaut")
}

fn err(text: &str) -> String {
    config::parse(text, &base()).expect_err("must be refused")
}

#[test]
fn a_full_file_parses_and_lowers() {
    let config = config::parse(FULL, &base()).expect("parses");
    assert_eq!(config.nick, "alice");
    assert_eq!(config.networks.len(), 2);
    // Read before the lowering, which is where the binary reads it too — the
    // account name does not survive into `NetworkSettings`, because the secret
    // it pairs with is not in this file.
    assert_eq!(
        config.networks["libera"].sasl_account.as_deref(),
        Some("alice")
    );
    assert_eq!(config.networks["ergo-local"].sasl_account, None);

    let networks = config.into_networks();
    let libera = networks
        .values()
        .find(|s| s.name == "libera")
        .expect("libera lowered");
    assert_eq!(libera.host, "irc.libera.chat");
    assert_eq!(libera.connection.nick, "alice");
    assert_eq!(libera.connection.username, "alice");
    assert_eq!(libera.connection.autojoin, ["#supernaut", "#rust"]);
    // Config holds only the account name (§5.8), so the lowering has no secret
    // to carry: the binary joins it to the keyring's half afterwards.
    assert!(libera.connection.sasl.is_none());
    assert!(matches!(libera.security, Security::Tls { .. }));

    let ergo = networks
        .values()
        .find(|s| s.name == "ergo-local")
        .expect("ergo lowered");
    assert!(matches!(ergo.security, Security::Plaintext));
    assert!(ergo.connection.autojoin.is_empty());
}

/// Ids come from the loader, sorted by name — never from an `id` key a human
/// types. Pinned because the ordering is the whole reason no `id` key exists.
#[test]
fn ids_are_one_to_n_in_name_order() {
    let text = r#"
nick = "alice"
[networks.zulu]
host = "z.example"
[networks.alpha]
host = "a.example"
[networks.mike]
host = "m.example"
"#;
    let networks = config::parse(text, &base())
        .expect("parses")
        .into_networks();
    assert_eq!(networks[&NetworkId(1)].name, "alpha");
    assert_eq!(networks[&NetworkId(2)].name, "mike");
    assert_eq!(networks[&NetworkId(3)].name, "zulu");
    assert_eq!(networks.len(), 3);
}

/// The invariant `announce` used to have to assume: two networks cannot share a
/// name, because the *file format* refuses it. This test is what makes that a
/// claim rather than a hope.
#[test]
fn a_duplicate_network_table_is_a_parse_error() {
    let message = err(r#"
nick = "alice"
[networks.libera]
host = "irc.libera.chat"
[networks.libera]
host = "other.example"
"#);
    assert!(
        message.contains("libera"),
        "the error must name the duplicated table: {message}"
    );
}

#[test]
fn an_unknown_key_errors() {
    let message = err(r#"
nick = "alice"
[networks.libera]
host = "irc.libera.chat"
hostname = "typo.example"
"#);
    assert!(
        message.contains("hostname"),
        "a silently-ignored typo is the bug class deny_unknown_fields exists for: {message}"
    );
    // Top-level too, not only inside a network table.
    let top = err("nick = \"alice\"\ntheme = \"dark\"\n[networks.libera]\nhost = \"h\"\n");
    assert!(top.contains("theme"), "{top}");
}

/// Refused **by name**, and the message has to point at where credentials do
/// live — the generic unknown-key error already rejects them, uselessly.
#[test]
fn credential_shaped_keys_are_refused_by_name() {
    for key in ["password", "pass", "sasl_password", "nickserv_password"] {
        let top = err(&format!(
            "nick = \"alice\"\n{key} = \"hunter2\"\n[networks.libera]\nhost = \"h\"\n"
        ));
        assert!(top.contains(key) && top.contains("keyring"), "{top}");
        let nested = err(&format!(
            "nick = \"alice\"\n[networks.libera]\nhost = \"h\"\n{key} = \"hunter2\"\n"
        ));
        assert!(
            nested.contains(key) && nested.contains("keyring"),
            "{nested}"
        );
    }
}

#[test]
fn plaintext_against_a_non_loopback_host_errors() {
    let message = err(r#"
nick = "alice"
[networks.bouncer]
host = "bnc.example.net"
plaintext = true
"#);
    assert!(
        message.contains("bouncer") && message.contains("bnc.example.net"),
        "the message must name the network and the host: {message}"
    );
    // The three loopback spellings are accepted, and only those.
    for host in ["localhost", "127.0.0.1", "::1"] {
        let text = format!("nick = \"a\"\n[networks.local]\nhost = \"{host}\"\nplaintext = true\n");
        config::parse(&text, &base()).unwrap_or_else(|e| panic!("{host} is loopback: {e}"));
    }
}

#[test]
fn plaintext_with_a_trust_anchor_errors() {
    let message = err(r#"
nick = "alice"
[networks.local]
host = "localhost"
plaintext = true
tls_ca = "ca.pem"
"#);
    assert!(
        message.contains("local") && message.contains("tls_ca"),
        "a trust anchor on a connection with no trust: {message}"
    );
}

#[test]
fn a_missing_empty_or_whitespace_nick_errors() {
    let missing = err("[networks.libera]\nhost = \"h\"\n");
    assert!(missing.contains("nick"), "{missing}");
    let empty = err("nick = \"\"\n[networks.libera]\nhost = \"h\"\n");
    assert!(empty.contains("nick"), "{empty}");
    // Whitespace breaks the NICK line itself — our bug, not the server's policy.
    let spaced = err("nick = \"al ice\"\n[networks.libera]\nhost = \"h\"\n");
    assert!(
        spaced.contains("nick") && spaced.contains("whitespace"),
        "{spaced}"
    );
}

/// Whitespace-only counts as empty for both: a `[networks." "]` table would
/// otherwise mint a `network` row named `" "` that nothing could refer to again.
#[test]
fn an_empty_host_or_network_name_errors() {
    let host = err("nick = \"a\"\n[networks.libera]\nhost = \"\"\n");
    assert!(host.contains("libera") && host.contains("host"), "{host}");
    let blank_host = err("nick = \"a\"\n[networks.libera]\nhost = \"  \"\n");
    assert!(
        blank_host.contains("libera") && blank_host.contains("host"),
        "{blank_host}"
    );
    let name = err("nick = \"a\"\n[networks.\"\"]\nhost = \"h\"\n");
    assert!(name.contains("empty name"), "{name}");
    let blank_name = err("nick = \"a\"\n[networks.\" \"]\nhost = \"h\"\n");
    assert!(blank_name.contains("empty name"), "{blank_name}");
}

/// A `nick`-only file is schema-valid — `networks` is `#[serde(default)]` — so
/// without this rule the refusal came from the binary and blamed `--network`,
/// which is the wrong thing to tell someone whose file has no network in it.
#[test]
fn a_config_naming_no_networks_errors() {
    let message = err("nick = \"alice\"\n");
    assert!(
        message.contains("no networks") && message.contains("networks."),
        "the error must name the missing table, not a flag: {message}"
    );
}

/// Channel *names* stay unvalidated (CHANTYPES is ISUPPORT's), but the **line**
/// is ours: entries are joined with `,` into one `JOIN`, so a space turns
/// `#my chan` into channel `#my` with key `chan` — a silently wrong join nobody
/// reports — and a CR/LF injects a second IRC command. Same rule as `nick`, same
/// reason.
#[test]
fn autojoin_entries_that_would_corrupt_the_join_line_error() {
    let spaced = err("nick = \"a\"\n[networks.n]\nhost = \"h\"\nautojoin = [\"#my chan\"]\n");
    assert!(
        spaced.contains("autojoin") && spaced.contains("#my chan") && spaced.contains("network n"),
        "the message must name the network and the entry: {spaced}"
    );
    let injected =
        err("nick = \"a\"\n[networks.n]\nhost = \"h\"\nautojoin = [\"#ok\\r\\nQUIT\"]\n");
    assert!(injected.contains("autojoin"), "{injected}");
    let empty = err("nick = \"a\"\n[networks.n]\nhost = \"h\"\nautojoin = [\"\"]\n");
    assert!(empty.contains("empty entry"), "{empty}");
    // A perfectly ordinary list still parses, including names we do not police.
    config::parse(
        "nick = \"a\"\n[networks.n]\nhost = \"h\"\nautojoin = [\"#a\", \"&b\", \"weird\"]\n",
        &base(),
    )
    .expect("names are the server's business, line integrity is ours");
}

/// The account name is framed into `authcid\0authcid\0password`, so a control
/// character splits our own payload — the same "the line is ours, the policy is
/// the server's" boundary `autojoin` and `nick` draw. What a network *permits*
/// as an account name stays unpoliced.
#[test]
fn a_sasl_account_that_would_corrupt_the_plain_payload_errors() {
    let spaced = err("nick = \"a\"\n[networks.n]\nhost = \"h\"\nsasl_account = \"al ice\"\n");
    assert!(
        spaced.contains("sasl_account") && spaced.contains("network n"),
        "the message must name the network and the key: {spaced}"
    );
    let injected = err("nick = \"a\"\n[networks.n]\nhost = \"h\"\nsasl_account = \"a\\u0000b\"\n");
    assert!(injected.contains("sasl_account"), "{injected}");
    let empty = err("nick = \"a\"\n[networks.n]\nhost = \"h\"\nsasl_account = \"  \"\n");
    assert!(
        empty.contains("sasl_account") && empty.contains("empty"),
        "{empty}"
    );
    // Odd but legal account names are the network's business, not ours.
    config::parse(
        "nick = \"a\"\n[networks.n]\nhost = \"h\"\nsasl_account = \"alice@example.net\"\n",
        &base(),
    )
    .expect("account-name syntax is the network's business");
}

/// SASL PLAIN puts the password on the wire; `plaintext` leaves the wire
/// readable. Stricter than §2.3 demands, exactly like the loopback rule beside
/// it — relaxing it is stage 2 item 7's product call.
#[test]
fn sasl_account_with_plaintext_errors() {
    let message = err(r#"
nick = "alice"
[networks.local]
host = "localhost"
plaintext = true
sasl_account = "alice"
"#);
    assert!(
        message.contains("sasl_account")
            && message.contains("plaintext")
            && message.contains("cleartext"),
        "the refusal must name both keys and the reason: {message}"
    );
}

/// `sasl_account` is a name, not a credential, so it is deliberately absent from
/// `CREDENTIAL_KEYS` — whose message says *credentials*, and would be lying if it
/// caught this key. Pinned, because "add it for symmetry" is the tempting bug.
#[test]
fn sasl_account_is_not_treated_as_a_credential_key() {
    config::parse(
        "nick = \"a\"\n[networks.n]\nhost = \"h\"\nsasl_account = \"alice\"\n",
        &base(),
    )
    .expect("an account name is not a secret");
    // While the four secret-shaped keys keep being refused beside it.
    let message = err(
        "nick = \"a\"\n[networks.n]\nhost = \"h\"\nsasl_account = \"a\"\nsasl_password = \"p\"\n",
    );
    assert!(
        message.contains("sasl_password") && message.contains("credential set"),
        "the refusal must point at the command that replaces the key: {message}"
    );
}

/// The refusal list must hold in *any* version of the schema, so it walks arrays
/// as well as tables — today's schema having no array-of-tables is a fact about
/// today, not a property to rely on.
#[test]
fn the_credential_scan_descends_through_arrays() {
    let message =
        err("nick = \"a\"\n[networks.n]\nhost = \"h\"\n[[extras]]\npassword = \"hunter2\"\n");
    assert!(
        message.contains("password") && message.contains("keyring"),
        "an array-of-tables must not smuggle one past the scan: {message}"
    );
}

/// Opinionated defaults are the product (§2.1): TLS 6697, plaintext 6667.
#[test]
fn the_two_default_ports() {
    let text = r#"
nick = "alice"
[networks.tls-default]
host = "irc.example.net"
[networks.plain-default]
host = "localhost"
plaintext = true
"#;
    let networks = config::parse(text, &base())
        .expect("parses")
        .into_networks();
    let port = |name: &str| {
        networks
            .values()
            .find(|s| s.name == name)
            .expect("lowered")
            .port
    };
    assert_eq!(port("tls-default"), 6697);
    assert_eq!(port("plain-default"), 6667);
}

/// Relative to the config file's own directory, so a generated fixture
/// directory is portable; absolute paths pass through untouched.
#[test]
fn tls_ca_resolves_against_the_config_directory() {
    let relative = config::parse(
        "nick = \"a\"\n[networks.n]\nhost = \"h\"\ntls_ca = \"certs/ca.pem\"\n",
        Path::new("/tmp/fixture-42"),
    )
    .expect("parses");
    assert_eq!(
        relative.networks["n"].tls_ca.as_deref(),
        Some(Path::new("/tmp/fixture-42/certs/ca.pem"))
    );

    let absolute = config::parse(
        "nick = \"a\"\n[networks.n]\nhost = \"h\"\ntls_ca = \"/etc/ssl/ca.pem\"\n",
        Path::new("/tmp/fixture-42"),
    )
    .expect("parses");
    assert_eq!(
        absolute.networks["n"].tls_ca.as_deref(),
        Some(Path::new("/etc/ssl/ca.pem"))
    );

    // And it survives the lowering into the Security the connector reads.
    let networks = relative.into_networks();
    match &networks[&NetworkId(1)].security {
        Security::Tls { ca_file, .. } => {
            assert_eq!(
                ca_file.as_deref(),
                Some(Path::new("/tmp/fixture-42/certs/ca.pem"))
            );
        }
        Security::Plaintext => panic!("no plaintext key was set"),
    }
}
