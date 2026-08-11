//! The credential store: SASL passwords in the OS keyring, and the
//! `credential set` subcommand that puts them there.
//!
//! **In the binary rather than in havoc-core, deliberately.** The credential
//! store is an OS integration belonging to whichever process assembles the core
//! — today `supernaut`, at stage 4 `havocd`, a binary either way — and core's
//! tests must not depend on the developer's login keychain. The type that
//! crosses the seam is [`SaslCredentials`], which core already owns.
//!
//! **Keyed by service `supernaut` and account = the config network's *name*.**
//! Never a [`havoc_ipc::NetworkId`]: `Config::into_networks` renumbers ids
//! `1..N` from sorted order, so a keyring entry keyed by id would be the first
//! thing in this program to persist a wire id, and it would silently re-point
//! one network's credentials at another's after an unrelated config edit. The
//! name is what storage keys on too.
//!
//! One entry per network, not `<network>:<account>`: `credential set <network>`
//! then needs no account argument, because config already holds the account, and
//! editing `sasl_account` cannot silently orphan a secret set five minutes ago —
//! a mismatched account fails loudly at the server instead, which is fail-closed.
//!
//! **What that costs, said out loud:** the keychain namespace is process-global,
//! so one `supernaut`/`liverun` item is shared by *every* `SUPERNAUT_CONFIG_DIR`
//! on the machine. Two config dirs naming a network `liverun` share one secret.
//! Hashing the config path into the service string would separate them, at the
//! price of an entry nobody can find in Keychain Access and nothing can share
//! with stage 4's daemon — rejected on those grounds (BUILD-LOG, 2026-08-10).
//!
//! **There is no encrypted-file fallback yet**, although NORTH-STAR §5.8 and
//! PLAN promise one: a fallback file needs a key, and every honest source of one
//! is out of stage 1's reach (see the decision entry). What ships instead is the
//! loud unavailability error in [`describe`] — absence has to be spoken rather
//! than discovered. It is filed on PLAN stage 4 item 3, deadline stage 6 item 3.

use std::io::{IsTerminal, Read};

use havoc_core::connection::SaslCredentials;

/// The keychain service every entry lives under. Deliberately the bare program
/// name: findable in Keychain Access, and shareable with stage 4's daemon.
const SERVICE: &str = "supernaut";

/// Read one network's SASL password out of the OS keyring and pair it with the
/// account name config supplied. Called for the **selected network only** — see
/// the comment at the call site in session.rs.
pub(crate) fn load(network: &str, account: &str) -> Result<SaslCredentials, String> {
    let password = entry(network)?
        .get_password()
        .map_err(|error| describe(network, &error))?;
    Ok(SaslCredentials {
        authcid: account.to_owned(),
        password,
    })
}

/// `supernaut credential set <network>`: read the secret from **stdin** and
/// store it. Never from argv — `ps` is world-readable, which is why prompt 6
/// refused a `--sasl-pass` flag; and never from a prompt, because suppressing
/// terminal echo is stage 2's first-run wizard's job (PLAN stage 2 item 7).
pub(crate) fn set(network_argument: &str) -> Result<(), String> {
    let config = crate::load_config()?;
    // The same sentence `session` uses, so a name the file does not hold gets one
    // error message in this binary rather than two.
    let network = crate::session::resolve_network(&config, Some(network_argument))?;
    let account = config.networks[&network]
        .sasl_account
        .clone()
        .ok_or_else(|| {
            format!(
                "config: network {network} has no `sasl_account`; add it to the \
                 [networks.{network}] table first, or the secret stored here would \
                 never be read"
            )
        })?;

    if std::io::stdin().is_terminal() {
        eprintln!(
            "reading the password from the terminal — it is NOT hidden. \
             Pipe it instead: printf %s 'secret' | supernaut credential set {network}"
        );
    }
    let mut secret = String::new();
    std::io::stdin()
        .read_to_string(&mut secret)
        .map_err(|error| format!("cannot read the password from stdin: {error}"))?;
    // Exactly one trailing newline, and nothing else: `printf '%s'` sends none
    // and a here-string or a terminal sends one, while anything further in is a
    // character of somebody's password and not ours to guess at.
    let secret = secret.strip_suffix('\n').unwrap_or(&secret);
    if secret.is_empty() {
        return Err(format!(
            "refusing to store an empty password for network {network}"
        ));
    }

    entry(&network)?
        .set_password(secret)
        .map_err(|error| describe(&network, &error))?;
    // Names the entry, never the secret (CLAUDE.md).
    println!(
        "stored a password in the OS keyring: service {SERVICE}, account {network} \
         (SASL account {account})"
    );
    Ok(())
}

fn entry(network: &str) -> Result<keyring::Entry, String> {
    keyring::Entry::new(SERVICE, network).map_err(|error| {
        // `Entry::new` collapses *every* store-initialization failure into a bare
        // `NoDefaultStore`, whose own message says only that there is no store. The
        // real cause — the D-Bus or Secret Service error on a headless Linux box, or
        // "platform not supported" — is held by `store_status()`, which caches the
        // one-time initialization result. Quote that instead, or the user who most
        // needs a true sentence gets the least informative one in the program.
        match (&error, keyring::Entry::store_status()) {
            (keyring::Error::NoDefaultStore, Err(cause)) => describe(network, cause),
            _ => describe(network, &error),
        }
    })
}

/// Three distinct sentences, because they call for three different actions, and
/// all of them are printed before anything dials.
fn describe(network: &str, error: &keyring::Error) -> String {
    match error {
        keyring::Error::NoEntry => format!(
            "no SASL password for network {network}: the OS keyring holds no item for \
             service {SERVICE}, account {network}. Store one:\n    \
             printf %s 'your-password' | supernaut credential set {network}"
        ),
        // Two items match, so no read can be unambiguous — and guessing between
        // them is exactly the wrong instinct for a credential.
        keyring::Error::Ambiguous(matches) => format!(
            "the OS keyring holds {} items for service {SERVICE}, account {network} — \
             open Keychain Access, search for {SERVICE}, and delete the duplicates of \
             the {network} entry until one is left",
            matches.len()
        ),
        // Everything else is the store itself, not this entry: a locked Secret
        // Service, a headless box with no store at all, a platform failure. The
        // true sentence matters more here than a short one.
        other => format!(
            "the OS keyring is unavailable ({other}), so network {network} cannot \
             authenticate: there is no encrypted-file fallback yet (deferred — PLAN \
             stage 4 item 3, due by stage 6 item 3). Remove `sasl_account` from the \
             [networks.{network}] table to connect without SASL."
        ),
    }
}
