//! Connection pool construction. A pool (rather than a single shared
//! connection behind a mutex) is used so a long-running operation on one
//! connection doesn't block every other database access — relevant once
//! recording/AI work runs concurrently with UI-driven reads (see
//! docs/architecture.md, "Concurrency").

use crate::errors::AppError;
use r2d2_sqlite::SqliteConnectionManager;
use std::path::Path;

pub type DbPool = r2d2::Pool<SqliteConnectionManager>;

/// Sensible desktop-app SQLite settings, applied to every pooled
/// connection when it's created:
/// - `foreign_keys = ON` — off by default in SQLite; we want referential
///   integrity enforced once real tables reference each other.
/// - `journal_mode = WAL` — allows concurrent readers while a write is in
///   progress, instead of the default rollback journal's stricter
///   locking.
/// - `busy_timeout` — instead of failing immediately when the database is
///   briefly locked by another connection, wait up to 5s.
const PRAGMAS: &str = "PRAGMA foreign_keys = ON;\nPRAGMA journal_mode = WAL;\nPRAGMA busy_timeout = 5000;";

pub fn build_pool(db_path: &Path) -> Result<DbPool, AppError> {
    let manager = SqliteConnectionManager::file(db_path)
        .with_init(|conn| conn.execute_batch(PRAGMAS));
    let pool = r2d2::Pool::builder().max_size(8).build(manager)?;
    Ok(pool)
}
