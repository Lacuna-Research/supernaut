//! The parts of storage that **are** disk format: the column encodings for the
//! two wire enums, and the synthetic-msgid hash. Split out of mod.rs and exec.rs
//! for the size ratchet, but the grouping is the point — a change to anything in
//! this file silently rewrites the meaning of history already written, so the
//! stability rule has one home rather than three.

use havoc_ipc::{BufferKind, MessageKind};

/// The stable `message.kind` column encoding of [`MessageKind`]. A deliberate
/// mapping, not a derive: the integers are a disk format, and reordering the
/// enum must not be able to silently rewrite history's meaning.
pub fn kind_code(kind: MessageKind) -> i64 {
    match kind {
        MessageKind::Privmsg => 0,
        MessageKind::Notice => 1,
        MessageKind::Join => 2,
        MessageKind::Part => 3,
        MessageKind::Quit => 4,
        MessageKind::Mode => 5,
        MessageKind::Topic => 6,
        MessageKind::Nick => 7,
        MessageKind::Server => 8,
    }
}

/// The inverse of [`kind_code`], for hydrating search hits. Loud on an
/// unknown code — a row this build cannot name is a bug, never a default.
pub fn kind_from_code(code: i64) -> Option<MessageKind> {
    Some(match code {
        0 => MessageKind::Privmsg,
        1 => MessageKind::Notice,
        2 => MessageKind::Join,
        3 => MessageKind::Part,
        4 => MessageKind::Quit,
        5 => MessageKind::Mode,
        6 => MessageKind::Topic,
        7 => MessageKind::Nick,
        8 => MessageKind::Server,
        _ => return None,
    })
}

/// The stable `buffer.kind` column encoding of [`BufferKind`] — matches the
/// snake_case the wire uses, by choice recorded here rather than by accident
/// of a serde attribute elsewhere.
pub fn buffer_kind_str(kind: BufferKind) -> &'static str {
    match kind {
        BufferKind::Channel => "channel",
        BufferKind::Query => "query",
        BufferKind::Server => "server",
        BufferKind::Special => "special",
    }
}

/// The inverse of [`buffer_kind_str`], for rows read back off disk.
pub(super) fn parse_kind(kind: &str) -> BufferKind {
    match kind {
        "channel" => BufferKind::Channel,
        "query" => BufferKind::Query,
        "server" => BufferKind::Server,
        _ => BufferKind::Special,
    }
}

/// FNV-1a 64, inline: the synthetic-msgid hash is disk format, so it must be
/// stable across releases (std's DefaultHasher is not) and sha2 fails the
/// dependency bar for one call site.
fn fnv1a64(bytes: &[u8]) -> u64 {
    let mut hash: u64 = 0xcbf2_9ce4_8422_2325;
    for byte in bytes {
        hash ^= u64::from(*byte);
        hash = hash.wrapping_mul(0x0000_0100_0000_01b3);
    }
    hash
}

/// §4.6's content-hash fallback for tagless servers: (nick, text, 30s
/// bucket). Imperfect by design — identical (nick, text) inside one bucket
/// collapses — and only ever used where nothing better exists.
pub(super) fn synthetic_msgid(nick: Option<&str>, text: Option<&str>, millis: i64) -> String {
    let bucket = millis / 30_000;
    let seed = format!(
        "{}\u{0}{}\u{0}{bucket}",
        nick.unwrap_or(""),
        text.unwrap_or("")
    );
    format!("fnv:{:016x}", fnv1a64(seed.as_bytes()))
}

#[cfg(test)]
mod tests {
    #[test]
    fn fnv_vectors_are_stable() {
        // Pinned: these values are disk format.
        assert_eq!(super::fnv1a64(b""), 0xcbf2_9ce4_8422_2325);
        assert_eq!(super::fnv1a64(b"a"), 0xaf63_dc4c_8601_ec8c);
        assert_eq!(
            super::synthetic_msgid(Some("alice"), Some("hello"), 61_000),
            super::synthetic_msgid(Some("alice"), Some("hello"), 75_000),
            "same 30s bucket must collapse"
        );
        assert_ne!(
            super::synthetic_msgid(Some("alice"), Some("hello"), 61_000),
            super::synthetic_msgid(Some("alice"), Some("hello"), 95_000),
            "different buckets must not"
        );
    }
}
