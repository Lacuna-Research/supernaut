//! The user's network seed file: the TOML schema, its validation, and the
//! lowering into the `HashMap<NetworkId, NetworkSettings>` the core is spawned
//! with. Config lives in havoc-core (PLAN's module table, NORTH-STAR §4.5) and
//! is **seed data only** — per the 2026-08-10 decision, the database owns
//! runtime state and the program never writes this file. That is not merely a
//! rule: `toml`'s `display` feature is off, so the serializer is not linked in
//! and the capability is absent.
//!
//! **No file I/O here, deliberately.** [`parse`] takes the text and the
//! directory the file came from, so every rule is testable without touching
//! disk. Locating and reading the file is the binary's job
//! (`default_config_path` in crates/supernaut/src/main.rs); parsing it is
//! core's.
//!
//! Errors are `String` rather than an enum for the same reason `search::parse`'s
//! are: nothing branches on the variant — the binary prints it and exits.
//!
//! The schema, in full:
//!
//! ```toml
//! # Seed data. Supernaut never writes this file.
//! nick = "alice"
//!
//! [networks.libera]
//! host = "irc.libera.chat"
//! autojoin = ["#supernaut"]
//! # The account name only. The password lives in the OS keyring:
//! #     printf %s 'secret' | supernaut credential set libera
//! sasl_account = "alice"
//!
//! [networks.ergo-local]
//! host = "localhost"
//! port = 6667
//! plaintext = true
//! ```

use std::collections::{BTreeMap, HashMap};
use std::path::{Path, PathBuf};

use havoc_ipc::NetworkId;

use crate::connection::Config as ConnectionConfig;
use crate::connection::io::Security;
use crate::core::NetworkSettings;

/// Not a config key. Diagnostics-grade text, carried verbatim from the
/// pre-config debug session; a per-network realname is stage 5's multi-network
/// work, and it is unobservable in a one-network stage.
const REALNAME: &str = "supernaut debug session";

/// Refused **by name**, in any table. `deny_unknown_fields` already rejects
/// them; what this list buys is a message that says credentials never live in
/// this file and names the two commands that replace the key the person just
/// typed — an instruction now that the keyring exists, where before prompt 10b
/// it could only be an aspiration (NORTH-STAR §5.8).
///
/// [`NetworkEntry::sasl_account`] is deliberately **not** on this list: an
/// account name is not a credential, and the message below says credentials, so
/// adding it would put a lie in the error a user reads.
const CREDENTIAL_KEYS: [&str; 4] = ["password", "pass", "sasl_password", "nickserv_password"];

/// The whole file. `deny_unknown_fields` is not optional: a silently-ignored
/// typo'd key is the config bug class that costs an evening.
#[derive(Debug, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Config {
    /// Global and single. `username` and `realname` are derived from it in the
    /// lowering; a per-network nick override waits until a second network makes
    /// it observable (PLAN stage 5 item 1).
    pub nick: String,
    /// Keyed by the **stable network name** — the same string `ensure_network`
    /// keys the `network` table on, which is what makes config identity and
    /// storage identity one thing rather than two that must agree.
    ///
    /// A map rather than `[[network]]` + `name` because two `[networks.libera]`
    /// tables are a TOML-level error: name uniqueness is enforced by the file
    /// format and is *unrepresentable* here, rather than being a validation pass
    /// somebody could forget to write. Names are case-sensitive, matching the
    /// `network.name TEXT UNIQUE` column's default collation — `Libera` and
    /// `libera` are two networks and two rows, deliberately.
    #[serde(default)]
    pub networks: BTreeMap<String, NetworkEntry>,
}

/// One network's seed data. No `id` (see [`Config::into_networks`]) and no
/// `server_name` for an SNI/connect-host split — that is one field when a
/// bouncer makes it real (PLAN stage 5 item 1).
#[derive(Debug, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct NetworkEntry {
    pub host: String,
    /// Defaults to 6697 with TLS and 6667 with plaintext — an opinionated
    /// default is the product (§2.1), not a field the user must fill.
    pub port: Option<u16>,
    /// Extra PEM trust anchor, resolved by [`parse`] relative to the config
    /// file's own directory, so a generated fixture directory is portable.
    /// Existence is deliberately **not** checked: the TLS connector reports
    /// that, once, where the failure actually is.
    pub tls_ca: Option<PathBuf>,
    /// Channel *names* are deliberately **not** validated — CHANTYPES comes from
    /// ISUPPORT, so we would be guessing, and the server's refusal is already
    /// loud — but **line integrity is**, for the same reason `nick` is: an entry
    /// is joined with `,` into one `JOIN` line, so whitespace or a control
    /// character in it corrupts the line we write, which is our bug and not the
    /// server's policy. `"#my chan"` would otherwise become `JOIN #my chan` —
    /// channel `#my` with key `chan`, a silently wrong join nobody reports — and
    /// a CR/LF would inject a second command outright.
    #[serde(default)]
    pub autojoin: Vec<String>,
    /// Authenticate with SASL PLAIN as this account. **The secret is not here,
    /// and cannot be** (§5.8): it comes from the OS keyring, keyed by this
    /// network's name — `supernaut credential set <network>` puts it there.
    /// Absent means no SASL at all, and no keyring access whatsoever.
    pub sasl_account: Option<String>,
    /// The loud opt-in (§2.3), and loopback-only — stricter than the north star
    /// asks, kept because it is the debug-grade rule stage 1 shipped; relaxing
    /// it is a product call belonging to stage 2's first-run. A **per-network
    /// key** rather than a process-global flag, because security is per-network
    /// and a flag cannot say which network it blesses.
    #[serde(default)]
    pub plaintext: bool,
}

/// Parse and validate, against a `base_dir` that is the config file's own
/// directory (what `tls_ca` resolves against). Every rule fires here, before
/// anything dials.
pub fn parse(text: &str, base_dir: &Path) -> Result<Config, String> {
    // Two passes over a file measured in lines, deliberately. The first is a
    // structure-blind scan for credential-shaped keys, so those get a message
    // naming the key instead of the generic unknown-field one. The second is
    // the real deserialize, whose errors carry the spans a hand-edited file
    // needs — which is why the scan does not simply deserialize from the value
    // tree it already has.
    let table: toml::Table = toml::from_str(text).map_err(|e| format!("config: {e}"))?;
    refuse_credential_keys(&table)?;
    let mut config: Config = toml::from_str(text).map_err(|e| format!("config: {e}"))?;
    config.validate(base_dir)?;
    Ok(config)
}

/// Depth-first, so `[networks.libera] password = "..."` is caught as surely as a
/// top-level one — and **through arrays as well as tables**, because the claim this
/// list makes is that the schema cannot acquire a credential key by accident *in
/// any version*, and today's schema having no array-of-tables is a fact about
/// today, not a property worth relying on.
fn refuse_credential_keys(table: &toml::Table) -> Result<(), String> {
    for (key, value) in table {
        if CREDENTIAL_KEYS.contains(&key.as_str()) {
            return Err(format!(
                "config: `{key}` is never a config key — SASL/NickServ credentials \
                 live in the OS keyring, never in plaintext in this file (NORTH-STAR §5.8). \
                 Set `sasl_account` in the network's table instead, and run \
                 `supernaut credential set <network>` to store the password."
            ));
        }
        refuse_in_value(value)?;
    }
    Ok(())
}

fn refuse_in_value(value: &toml::Value) -> Result<(), String> {
    if let Some(table) = value.as_table() {
        return refuse_credential_keys(table);
    }
    if let Some(array) = value.as_array() {
        for element in array {
            refuse_in_value(element)?;
        }
    }
    Ok(())
}

impl Config {
    /// Every message names the network and the key, because a config error a
    /// person cannot locate in their own file is barely an error at all.
    fn validate(&mut self, base_dir: &Path) -> Result<(), String> {
        if self.nick.is_empty() {
            return Err("config: `nick` must not be empty".to_owned());
        }
        if self.nick.chars().any(char::is_whitespace) {
            return Err(format!(
                "config: `nick` must not contain whitespace (got {:?}) — \
                 whitespace breaks the NICK line itself",
                self.nick
            ));
        }
        if self.networks.is_empty() {
            return Err(
                "config: no networks; add a [networks.<name>] table with a `host` key".to_owned(),
            );
        }
        for (name, entry) in &mut self.networks {
            // `trim()`, not `is_empty()`: a whitespace-only name would mint a
            // `network` row called " " that nothing could ever refer to again.
            if name.trim().is_empty() {
                return Err("config: a network table has an empty name".to_owned());
            }
            if entry.host.trim().is_empty() {
                return Err(format!("config: network {name}: `host` must not be empty"));
            }
            for channel in &entry.autojoin {
                if channel.is_empty() {
                    return Err(format!(
                        "config: network {name}: `autojoin` has an empty entry"
                    ));
                }
                if channel.chars().any(|c| c.is_whitespace() || c.is_control()) {
                    return Err(format!(
                        "config: network {name}: `autojoin` entry {channel:?} must not \
                         contain whitespace or control characters — it is joined into one \
                         JOIN line, so a space makes it a channel key and a newline makes \
                         it a second command"
                    ));
                }
            }
            if let Some(account) = &entry.sasl_account {
                // What a network *permits* as an account name is the network's
                // business — policing it would be guessing, exactly as with
                // channel names. What is ours is the payload we frame: SASL
                // PLAIN sends `authcid\0authcid\0password`, so a control
                // character splits or corrupts our own payload, and whitespace
                // is a quoting mistake worth naming for free.
                if account.trim().is_empty() {
                    return Err(format!(
                        "config: network {name}: `sasl_account` must not be empty — \
                         remove the key to connect without SASL"
                    ));
                }
                if account.chars().any(|c| c.is_whitespace() || c.is_control()) {
                    return Err(format!(
                        "config: network {name}: `sasl_account` {account:?} must not contain \
                         whitespace or control characters — it is framed into the SASL PLAIN \
                         payload `account\\0account\\0password`, which a control character \
                         splits and a space is almost always a quoting mistake in"
                    ));
                }
                // Stricter than §2.3 demands, like the loopback rule below it:
                // relaxing this is stage 2 item 7's product call, where the
                // plaintext-LAN user first appears.
                if entry.plaintext {
                    return Err(format!(
                        "config: network {name}: `sasl_account` with `plaintext` would send \
                         the password in cleartext — SASL PLAIN puts the secret on the wire, \
                         and plaintext leaves it readable. Drop one of the two keys."
                    ));
                }
            }
            if entry.plaintext {
                if entry.tls_ca.is_some() {
                    return Err(format!(
                        "config: network {name}: `plaintext` with `tls_ca` asks for a \
                         trust anchor on a connection that has no trust"
                    ));
                }
                if !is_loopback(&entry.host) {
                    return Err(format!(
                        "config: network {name}: `plaintext` permits loopback only; \
                         {} is not 127.0.0.1/::1/localhost",
                        entry.host
                    ));
                }
            }
            if let Some(ca) = entry.tls_ca.take() {
                entry.tls_ca = Some(base_dir.join(ca));
            }
        }
        Ok(())
    }

    /// The lowering: networks **sorted by name** get `NetworkId(1..N)`.
    ///
    /// Ids are caller-assigned in the sense core.rs means — assigned outside the
    /// engine, and a distinct type from the storage row id — but assigned by
    /// this loader rather than typed by a human: hand-numbering networks is
    /// exactly the config ceremony §2.1 calls a failed product, so there is no
    /// `id` key. Renumbering across runs is unobservable because **no wire
    /// `NetworkId` is ever persisted**: `ensure_network` keys the `network` table
    /// on the name and `buffer.network_id` references the storage row. If a wire
    /// `NetworkId` ever gets persisted or cached across restarts, the id becomes
    /// a config field, with the uniqueness rule and the migration story that
    /// implies.
    ///
    /// `sasl` lowers to `None`, always: config holds only half a credential
    /// (§5.8), so there is nothing here to lower. The binary joins the two
    /// halves afterwards, for the selected network alone — `sasl_account` from
    /// this same file, the password from the OS keyring keyed by the network's
    /// name (`crates/supernaut/src/credentials.rs`).
    #[must_use]
    pub fn into_networks(self) -> HashMap<NetworkId, NetworkSettings> {
        let nick = self.nick;
        self.networks
            .into_iter()
            .zip(1i64..)
            .map(|((name, entry), ordinal)| {
                let port = entry
                    .port
                    .unwrap_or(if entry.plaintext { 6667 } else { 6697 });
                let security = if entry.plaintext {
                    Security::Plaintext
                } else {
                    Security::Tls {
                        server_name: entry.host.clone(),
                        ca_file: entry.tls_ca,
                    }
                };
                (
                    NetworkId(ordinal),
                    NetworkSettings {
                        name,
                        host: entry.host,
                        port,
                        security,
                        connection: ConnectionConfig {
                            nick: nick.clone(),
                            username: nick.clone(),
                            realname: REALNAME.to_owned(),
                            sasl: None,
                            autojoin: entry.autojoin,
                        },
                    },
                )
            })
            .collect()
    }
}

/// A string check, not name resolution — unchanged from the pre-config session,
/// where it lived beside the `--allow-plaintext` flag this key replaced.
fn is_loopback(host: &str) -> bool {
    host == "localhost" || host == "127.0.0.1" || host == "::1"
}
