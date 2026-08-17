//! Versioned schema migrations, applied automatically on startup.
//!
//! Deliberately hand-rolled rather than pulling in a migration crate: the
//! whole mechanism is "track a version number, apply the SQL files newer
//! than it, in order" — a few lines built on SQLite's own `user_version`
//! pragma (an integer SQLite reserves exactly for this purpose). This
//! keeps the dependency footprint minimal, which matters here in
//! particular since `rusqlite_migration` currently has no version that is
//! both compatible with our `rusqlite`/`libsqlite3-sys` versions and this
//! project's Rust toolchain.
//!
//! Each migration file is embedded into the binary at compile time
//! (`include_str!`) and applied inside its own transaction, so a failure
//! partway through a migration doesn't leave the schema half-applied.
//! Applying the same migrations twice is a no-op — only files whose
//! version is greater than the database's current `user_version` run — so
//! this is safe to call on every application startup.
//!
//! Once a migration has shipped, it must never be edited — add a new
//! numbered file and register it below instead.

use crate::errors::AppError;
use rusqlite::Connection;

/// `(version, sql)` pairs, in ascending order. `version` becomes the
/// database's `user_version` once the migration has been applied.
const MIGRATIONS: &[(i64, &str)] = &[(1, include_str!("../../migrations/0001_initial.sql"))];

pub fn run(conn: &mut Connection) -> Result<(), AppError> {
    let current_version: i64 = conn
        .query_row("PRAGMA user_version", [], |row| row.get(0))
        .map_err(|err| {
            eprintln!("[golive] failed to read schema version: {err}");
            AppError::Migration
        })?;

    for (version, sql) in MIGRATIONS {
        if *version <= current_version {
            continue;
        }

        let tx = conn.transaction().map_err(|err| {
            eprintln!("[golive] failed to start migration transaction for version {version}: {err}");
            AppError::Migration
        })?;

        tx.execute_batch(sql).map_err(|err| {
            eprintln!("[golive] migration {version} failed: {err}");
            AppError::Migration
        })?;

        // PRAGMA statements don't accept bound parameters; the version
        // comes from our own fixed `MIGRATIONS` table above, not external
        // input, so formatting it into the SQL string is safe here.
        tx.execute_batch(&format!("PRAGMA user_version = {version}"))
            .map_err(|err| {
                eprintln!("[golive] failed to record schema version {version}: {err}");
                AppError::Migration
            })?;

        tx.commit().map_err(|err| {
            eprintln!("[golive] failed to commit migration {version}: {err}");
            AppError::Migration
        })?;
    }

    Ok(())
}
