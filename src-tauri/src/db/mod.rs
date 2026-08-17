//! The application-level database service: owns database initialization,
//! the connection pool, and migrations. Tauri commands never open a SQLite
//! connection or run migrations themselves — they go through the pool
//! this type provides (see docs/architecture.md, "Persistence").
//!
//! Deliberately has no dependency on Tauri types: it takes a plain
//! `&Path` for the app-data directory, so it's fully unit-testable with a
//! temp directory (see tests below) without spinning up a Tauri app.

mod migrations;
mod pool;

pub use pool::DbPool;

use crate::errors::AppError;
use std::path::Path;

pub struct DbService {
    pool: DbPool,
}

impl DbService {
    /// Resolves `<data_dir>/database/golive.db` (creating the `database`
    /// directory if needed), opens/creates the SQLite file, applies
    /// pending migrations, and returns a service holding a ready
    /// connection pool. Idempotent: safe to call on every application
    /// startup — it neither recreates nor destroys an existing database.
    pub fn init(data_dir: &Path) -> Result<Self, AppError> {
        let db_dir = data_dir.join("database");
        std::fs::create_dir_all(&db_dir)?;
        let db_path = db_dir.join("golive.db");

        let pool = pool::build_pool(&db_path)?;
        {
            let mut conn = pool.get()?;
            migrations::run(&mut conn)?;
        }

        Ok(Self { pool })
    }

    pub fn pool(&self) -> DbPool {
        self.pool.clone()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn init_creates_database_directory_and_file() {
        let dir = tempfile::tempdir().expect("temp dir");
        let db = DbService::init(dir.path()).expect("db init should succeed");
        assert!(dir.path().join("database").join("golive.db").exists());
        drop(db);
    }

    #[test]
    fn init_is_idempotent_across_repeated_startups() {
        let dir = tempfile::tempdir().expect("temp dir");

        let db1 = DbService::init(dir.path()).expect("first init should succeed");
        drop(db1);

        // Simulates closing and relaunching the application: running
        // init again against the same directory must not fail, recreate,
        // or wipe the database.
        let db2 = DbService::init(dir.path()).expect("second init should succeed");
        drop(db2);

        let db3 = DbService::init(dir.path()).expect("third init should succeed");
        drop(db3);
    }

    #[test]
    fn schema_created_by_migrations_is_queryable() {
        let dir = tempfile::tempdir().expect("temp dir");
        let db = DbService::init(dir.path()).expect("db init should succeed");
        let conn = db.pool().get().expect("pooled connection");
        let count: i64 = conn
            .query_row(
                "SELECT count(*) FROM sqlite_master WHERE type = 'table' AND name = 'app_metadata'",
                [],
                |row| row.get(0),
            )
            .expect("querying sqlite_master should succeed");
        assert_eq!(count, 1, "app_metadata table should exist after migrations");
    }

    #[test]
    fn processes_table_and_indexes_exist_after_migrations() {
        let dir = tempfile::tempdir().expect("temp dir");
        let db = DbService::init(dir.path()).expect("db init should succeed");
        let conn = db.pool().get().expect("pooled connection");

        let table_count: i64 = conn
            .query_row(
                "SELECT count(*) FROM sqlite_master WHERE type = 'table' AND name = 'processes'",
                [],
                |row| row.get(0),
            )
            .expect("querying sqlite_master should succeed");
        assert_eq!(table_count, 1, "processes table should exist after migrations");

        let index_count: i64 = conn
            .query_row(
                "SELECT count(*) FROM sqlite_master WHERE type = 'index' AND tbl_name = 'processes'",
                [],
                |row| row.get(0),
            )
            .expect("querying sqlite_master should succeed");
        assert!(
            index_count >= 2,
            "expected at least the project_id and project_id+updated_at indexes"
        );
    }

    #[test]
    fn processes_foreign_key_to_projects_is_declared() {
        let dir = tempfile::tempdir().expect("temp dir");
        let db = DbService::init(dir.path()).expect("db init should succeed");
        let conn = db.pool().get().expect("pooled connection");

        let fk_target: String = conn
            .query_row("SELECT \"table\" FROM pragma_foreign_key_list('processes')", [], |row| {
                row.get(0)
            })
            .expect("processes should declare a foreign key");
        assert_eq!(fk_target, "projects");
    }
}
