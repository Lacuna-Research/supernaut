//! Hand-rolled versioned migrations keyed on `PRAGMA user_version`, per the
//! 2026-08-09 decision (refinery rejected: a dependency and its own bookkeeping
//! table for what user_version already provides).
//!
//! Rules: migrations are numbered, embedded, applied in order, each in its own
//! transaction, and immutable once merged. `user_version` after a successful
//! run equals the migration count.

use rusqlite::Connection;

/// Ordered and append-only. Index 0 runs first; `user_version` records how
/// many entries have been applied.
const MIGRATIONS: &[&str] = &[
    include_str!("../../migrations/0001_init.sql"),
    include_str!("../../migrations/0002_fts.sql"),
];

/// What `migrate` did, so callers can make it observable.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MigrationReport {
    pub from_version: i64,
    pub to_version: i64,
}

impl MigrationReport {
    pub fn applied(&self) -> i64 {
        self.to_version - self.from_version
    }
}

pub fn migrate(conn: &mut Connection) -> Result<MigrationReport, super::StorageError> {
    let from_version: i64 = conn.query_row("PRAGMA user_version", [], |row| row.get(0))?;
    let target = i64::try_from(MIGRATIONS.len()).expect("migration count fits i64");

    if from_version > target {
        return Err(super::StorageError::FutureSchema {
            found: from_version,
            supported: target,
        });
    }

    for (index, sql) in MIGRATIONS
        .iter()
        .enumerate()
        .skip(usize::try_from(from_version).expect("non-negative"))
    {
        let version = i64::try_from(index).expect("fits") + 1;
        let tx = conn.transaction()?;
        tx.execute_batch(sql)?;
        // PRAGMA cannot be parameterized; version is derived from a constant's
        // index, not from input.
        tx.execute_batch(&format!("PRAGMA user_version = {version}"))?;
        tx.commit()?;
    }

    Ok(MigrationReport {
        from_version,
        to_version: target,
    })
}
