//! The concrete repository boundary this task introduces, per
//! docs/architecture.md's persistence pattern (domain/application logic
//! -> repository trait -> SQLite implementation). Deliberately scoped to
//! exactly the one operation needed to prove persistence — not a generic
//! `Repository<T>` — since there is no real domain model yet (that's
//! TASK-005).

use crate::db::DbPool;
use crate::errors::AppError;
use rusqlite::{params, Connection};

/// The fixed key under which the persistence-proof marker is stored in
/// `app_metadata`. Its value is set exactly once — the first time it's
/// requested — and never overwritten, so returning the same value across
/// application restarts is the proof that data survives in SQLite.
const STORAGE_MARKER_KEY: &str = "storage_initialized_at";

pub trait StorageStatusRepository: Send + Sync {
    /// Returns the persistence marker, writing it first if it doesn't
    /// exist yet. The value is a Unix timestamp (seconds) of when local
    /// storage was first initialized on this machine.
    fn ensure_marker(&self) -> Result<String, AppError>;
}

pub struct SqliteStorageStatusRepository {
    pool: DbPool,
}

impl SqliteStorageStatusRepository {
    pub fn new(pool: DbPool) -> Self {
        Self { pool }
    }

    fn read_marker(conn: &Connection) -> Result<Option<String>, AppError> {
        let mut stmt = conn.prepare("SELECT value FROM app_metadata WHERE key = ?1")?;
        let mut rows = stmt.query(params![STORAGE_MARKER_KEY])?;
        match rows.next()? {
            Some(row) => Ok(Some(row.get(0)?)),
            None => Ok(None),
        }
    }
}

impl StorageStatusRepository for SqliteStorageStatusRepository {
    fn ensure_marker(&self) -> Result<String, AppError> {
        let conn = self.pool.get()?;

        if let Some(value) = Self::read_marker(&conn)? {
            return Ok(value);
        }

        let value = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0)
            .to_string();

        conn.execute(
            "INSERT INTO app_metadata (key, value) VALUES (?1, ?2)
             ON CONFLICT(key) DO NOTHING",
            params![STORAGE_MARKER_KEY, value],
        )?;

        // Re-read rather than trusting `value`: if another connection won
        // a race and inserted first, this returns the value that actually
        // persisted, which is the one that matters.
        Self::read_marker(&conn)?.ok_or(AppError::Database)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::DbService;

    fn repo_in_temp_dir() -> (tempfile::TempDir, SqliteStorageStatusRepository) {
        let dir = tempfile::tempdir().expect("temp dir");
        let db = DbService::init(dir.path()).expect("db init");
        let repo = SqliteStorageStatusRepository::new(db.pool());
        (dir, repo)
    }

    #[test]
    fn ensure_marker_writes_then_returns_a_value() {
        let (_dir, repo) = repo_in_temp_dir();
        let value = repo.ensure_marker().expect("ensure_marker should succeed");
        assert!(!value.is_empty());
    }

    #[test]
    fn ensure_marker_returns_the_same_value_on_repeated_calls() {
        let (_dir, repo) = repo_in_temp_dir();
        let first = repo.ensure_marker().expect("first call");
        let second = repo.ensure_marker().expect("second call");
        let third = repo.ensure_marker().expect("third call");
        assert_eq!(first, second);
        assert_eq!(second, third);
    }

    #[test]
    fn marker_survives_reopening_the_database() {
        let dir = tempfile::tempdir().expect("temp dir");

        let written = {
            let db = DbService::init(dir.path()).expect("db init (first run)");
            let repo = SqliteStorageStatusRepository::new(db.pool());
            repo.ensure_marker().expect("write marker")
        }; // `db` (and its pool) dropped here — simulates closing the app.

        let read_back = {
            let db = DbService::init(dir.path()).expect("db init (second run)");
            let repo = SqliteStorageStatusRepository::new(db.pool());
            repo.ensure_marker().expect("read marker back")
        }; // simulates relaunching the app against the same data directory.

        assert_eq!(
            written, read_back,
            "the marker written on the first run must be exactly what's read on the next run"
        );
    }
}
