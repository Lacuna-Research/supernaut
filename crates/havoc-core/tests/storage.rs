//! Storage tests against real temp-file SQLite — never mocked (PLAN testing
//! strategy): it is embedded and fast, and a mock would test the mock.

use havoc_core::storage::{MigrationReport, Storage, buffer_kind_str, kind_code};
use havoc_ipc::{BufferKind, MessageKind};

fn temp_db() -> (tempdir::Dir, std::path::PathBuf) {
    let dir = tempdir::Dir::new();
    let path = dir.path().join("history.db");
    (dir, path)
}

/// Minimal RAII temp dir so the test suite adds no dependency for it.
mod tempdir {
    use std::path::{Path, PathBuf};
    use std::sync::atomic::{AtomicU64, Ordering};

    static COUNTER: AtomicU64 = AtomicU64::new(0);

    pub struct Dir(PathBuf);

    impl Dir {
        pub fn new() -> Self {
            let unique = format!(
                "havoc-storage-test-{}-{}",
                std::process::id(),
                COUNTER.fetch_add(1, Ordering::Relaxed)
            );
            let path = std::env::temp_dir().join(unique);
            std::fs::create_dir_all(&path).expect("create temp dir");
            Self(path)
        }

        pub fn path(&self) -> &Path {
            &self.0
        }
    }

    impl Drop for Dir {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.0);
        }
    }
}

#[test]
fn migrations_create_then_noop() {
    let (_dir, path) = temp_db();

    let (storage, report) = Storage::open(&path, false).expect("first open");
    assert_eq!(
        report,
        MigrationReport {
            from_version: 0,
            to_version: 1
        }
    );
    assert_eq!(report.applied(), 1);
    assert_eq!(storage.client().schema_version().expect("version"), 1);
    drop(storage);

    let (storage, report) = Storage::open(&path, false).expect("second open");
    assert_eq!(report.applied(), 0, "reopening must be a no-op");
    assert_eq!(storage.client().schema_version().expect("version"), 1);
}

/// Drift from the §4.9 shape must be loud: assert the exact schema.
#[test]
fn schema_matches_north_star() {
    let (_dir, path) = temp_db();
    let (storage, _) = Storage::open(&path, false).expect("open");
    drop(storage); // release the write lock before inspecting

    let conn = rusqlite::Connection::open(&path).expect("inspect");
    let mut names: Vec<String> = conn
        .prepare("SELECT name FROM sqlite_master WHERE type IN ('table','index') AND name NOT LIKE 'sqlite_%' ORDER BY name")
        .expect("prepare")
        .query_map([], |row| row.get(0))
        .expect("query")
        .map(|r| r.expect("row"))
        .collect();
    names.sort();
    assert_eq!(
        names,
        ["buffer", "message", "msg_msgid", "msg_time", "network"]
    );

    let sql_of = |name: &str| -> String {
        conn.query_row(
            "SELECT sql FROM sqlite_master WHERE name = ?1",
            [name],
            |row| row.get(0),
        )
        .expect("object sql")
    };

    let message_sql = sql_of("message");
    assert!(
        message_sql.contains("WITHOUT ROWID"),
        "message must be WITHOUT ROWID"
    );
    assert!(message_sql.contains("PRIMARY KEY (buffer_id, seq)"));
    for column in [
        "buffer_id   INTEGER NOT NULL",
        "seq         INTEGER NOT NULL",
        "msgid       TEXT",
        "server_time INTEGER NOT NULL",
        "kind        INTEGER NOT NULL",
        "nick        TEXT",
        "account     TEXT",
        "text        TEXT",
        "tags        BLOB",
    ] {
        assert!(message_sql.contains(column), "message must carry: {column}");
    }

    let msgid_sql = sql_of("msg_msgid");
    assert!(msgid_sql.contains("UNIQUE"), "msgid index must be unique");
    assert!(msgid_sql.contains("(buffer_id, msgid)"));
    assert!(
        msgid_sql.contains("WHERE msgid IS NOT NULL"),
        "dropping the partial predicate would break dedup silently"
    );
    assert!(sql_of("msg_time").contains("(buffer_id, server_time)"));
    assert!(sql_of("buffer").contains("UNIQUE (network_id, name)"));

    let journal: String = conn
        .query_row("PRAGMA journal_mode", [], |row| row.get(0))
        .expect("journal_mode");
    assert_eq!(journal.to_lowercase(), "wal");
}

#[test]
fn ensure_calls_are_idempotent_through_the_channel() {
    let (_dir, path) = temp_db();
    let (storage, _) = Storage::open(&path, false).expect("open");

    let client = storage.client();
    let network = client.ensure_network("libera").expect("network");
    let network_again = client.ensure_network("libera").expect("idempotent");
    assert_eq!(network, network_again);

    let buffer = client
        .ensure_buffer(network, "#supernaut", BufferKind::Channel)
        .expect("buffer");
    assert_eq!(
        client
            .ensure_buffer(network, "#supernaut", BufferKind::Channel)
            .expect("idempotent"),
        buffer
    );
}

/// The disk encodings are a format, not an implementation detail — pin them.
#[test]
fn kind_encodings_are_stable() {
    assert_eq!(kind_code(MessageKind::Privmsg), 0);
    assert_eq!(kind_code(MessageKind::Notice), 1);
    assert_eq!(kind_code(MessageKind::Join), 2);
    assert_eq!(kind_code(MessageKind::Part), 3);
    assert_eq!(kind_code(MessageKind::Quit), 4);
    assert_eq!(kind_code(MessageKind::Mode), 5);
    assert_eq!(kind_code(MessageKind::Topic), 6);
    assert_eq!(kind_code(MessageKind::Nick), 7);
    assert_eq!(kind_code(MessageKind::Server), 8);
    assert_eq!(buffer_kind_str(BufferKind::Channel), "channel");
    assert_eq!(buffer_kind_str(BufferKind::Query), "query");
    assert_eq!(buffer_kind_str(BufferKind::Server), "server");
    assert_eq!(buffer_kind_str(BufferKind::Special), "special");
}
