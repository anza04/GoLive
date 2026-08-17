//! `ProcessRepository`: the persistence boundary for the Process domain
//! (docs/architecture.md, "Persistence boundary"). All SQL for processes
//! lives here. No business rules — project-existence validation lives in
//! `ProcessService`, not here.

use crate::db::DbPool;
use crate::errors::AppError;
use crate::models::process::Process;
use rusqlite::{params, Row};

pub trait ProcessRepository: Send + Sync {
    fn create(&self, process: &Process) -> Result<(), AppError>;
    /// Ordered `updated_at DESC` — most recently touched process first.
    /// Only processes belonging to `project_id` are returned.
    fn list_by_project(&self, project_id: &str) -> Result<Vec<Process>, AppError>;
    fn get(&self, id: &str) -> Result<Option<Process>, AppError>;
    /// Writes `name`, `description`, `status`, and `updated_at`. `id`,
    /// `project_id`, and `created_at` are not part of the `SET` clause —
    /// structurally impossible to change through this method. A Process
    /// can never be moved to a different project this way.
    /// Returns whether a row was actually updated (`false` if `id` didn't
    /// exist), so the caller can distinguish "updated" from "not found".
    fn update(&self, process: &Process) -> Result<bool, AppError>;
    /// Returns whether a row was actually deleted (`false` if `id` didn't
    /// exist).
    fn delete(&self, id: &str) -> Result<bool, AppError>;
}

pub struct SqliteProcessRepository {
    pool: DbPool,
}

impl SqliteProcessRepository {
    pub fn new(pool: DbPool) -> Self {
        Self { pool }
    }
}

const SELECT_COLUMNS: &str = "id, project_id, name, description, status, created_at, updated_at";

fn row_to_process(row: &Row) -> rusqlite::Result<Process> {
    Ok(Process {
        id: row.get(0)?,
        project_id: row.get(1)?,
        name: row.get(2)?,
        description: row.get(3)?,
        status: row.get(4)?,
        created_at: row.get(5)?,
        updated_at: row.get(6)?,
    })
}

impl ProcessRepository for SqliteProcessRepository {
    fn create(&self, process: &Process) -> Result<(), AppError> {
        let conn = self.pool.get()?;
        conn.execute(
            "INSERT INTO processes (id, project_id, name, description, status, created_at, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
            params![
                process.id,
                process.project_id,
                process.name,
                process.description,
                process.status,
                process.created_at,
                process.updated_at,
            ],
        )?;
        Ok(())
    }

    fn list_by_project(&self, project_id: &str) -> Result<Vec<Process>, AppError> {
        let conn = self.pool.get()?;
        let mut stmt = conn.prepare(&format!(
            "SELECT {SELECT_COLUMNS} FROM processes WHERE project_id = ?1 ORDER BY updated_at DESC"
        ))?;
        let rows = stmt.query_map(params![project_id], row_to_process)?;
        let mut processes = Vec::new();
        for row in rows {
            processes.push(row?);
        }
        Ok(processes)
    }

    fn get(&self, id: &str) -> Result<Option<Process>, AppError> {
        let conn = self.pool.get()?;
        let mut stmt = conn.prepare(&format!("SELECT {SELECT_COLUMNS} FROM processes WHERE id = ?1"))?;
        let mut rows = stmt.query(params![id])?;
        match rows.next()? {
            Some(row) => Ok(Some(row_to_process(row)?)),
            None => Ok(None),
        }
    }

    fn update(&self, process: &Process) -> Result<bool, AppError> {
        let conn = self.pool.get()?;
        let affected = conn.execute(
            "UPDATE processes SET name = ?1, description = ?2, status = ?3, updated_at = ?4 WHERE id = ?5",
            params![
                process.name,
                process.description,
                process.status,
                process.updated_at,
                process.id,
            ],
        )?;
        Ok(affected > 0)
    }

    fn delete(&self, id: &str) -> Result<bool, AppError> {
        let conn = self.pool.get()?;
        let affected = conn.execute("DELETE FROM processes WHERE id = ?1", params![id])?;
        Ok(affected > 0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::DbService;
    use crate::models::process::ProcessStatus;
    use crate::models::project::Project;
    use crate::repositories::project::{ProjectRepository, SqliteProjectRepository};

    fn repos_in_temp_dir() -> (tempfile::TempDir, SqliteProcessRepository, SqliteProjectRepository) {
        let dir = tempfile::tempdir().expect("temp dir");
        let db = DbService::init(dir.path()).expect("db init");
        (
            dir,
            SqliteProcessRepository::new(db.pool()),
            SqliteProjectRepository::new(db.pool()),
        )
    }

    fn sample_project(id: &str) -> Project {
        Project {
            id: id.to_string(),
            name: "Sample project".to_string(),
            description: String::new(),
            created_at: 1,
            updated_at: 1,
        }
    }

    fn sample_process(id: &str, project_id: &str, name: &str, updated_at: i64) -> Process {
        Process {
            id: id.to_string(),
            project_id: project_id.to_string(),
            name: name.to_string(),
            description: "desc".to_string(),
            status: ProcessStatus::Draft,
            created_at: updated_at,
            updated_at,
        }
    }

    #[test]
    fn create_then_get_round_trips() {
        let (_dir, repo, projects) = repos_in_temp_dir();
        projects.create(&sample_project("proj1")).unwrap();

        let process = sample_process("p1", "proj1", "Create a sales order", 1000);
        repo.create(&process).expect("create");

        let fetched = repo.get("p1").expect("get").expect("process should exist");
        assert_eq!(fetched.id, process.id);
        assert_eq!(fetched.project_id, "proj1");
        assert_eq!(fetched.name, process.name);
        assert_eq!(fetched.description, process.description);
        assert_eq!(fetched.status, ProcessStatus::Draft);
    }

    #[test]
    fn get_missing_process_returns_none() {
        let (_dir, repo, _projects) = repos_in_temp_dir();
        assert!(repo.get("does-not-exist").expect("get").is_none());
    }

    #[test]
    fn list_by_project_returns_only_matching_processes() {
        let (_dir, repo, projects) = repos_in_temp_dir();
        projects.create(&sample_project("proj1")).unwrap();
        projects.create(&sample_project("proj2")).unwrap();
        repo.create(&sample_process("p1", "proj1", "Process A", 1000)).unwrap();
        repo.create(&sample_process("p2", "proj2", "Process B", 2000)).unwrap();

        let listed = repo.list_by_project("proj1").expect("list");
        assert_eq!(listed.len(), 1);
        assert_eq!(listed[0].id, "p1");
    }

    #[test]
    fn list_by_project_orders_by_updated_at_descending() {
        let (_dir, repo, projects) = repos_in_temp_dir();
        projects.create(&sample_project("proj1")).unwrap();
        repo.create(&sample_process("older", "proj1", "Older", 1000)).unwrap();
        repo.create(&sample_process("newer", "proj1", "Newer", 2000)).unwrap();

        let listed = repo.list_by_project("proj1").expect("list");
        let ids: Vec<&str> = listed.iter().map(|p| p.id.as_str()).collect();
        assert_eq!(ids, vec!["newer", "older"]);
    }

    #[test]
    fn update_changes_name_description_and_status() {
        let (_dir, repo, projects) = repos_in_temp_dir();
        projects.create(&sample_project("proj1")).unwrap();
        let process = sample_process("p1", "proj1", "Original", 1000);
        repo.create(&process).unwrap();

        let mut changed = process.clone();
        changed.name = "Updated".to_string();
        changed.description = "new desc".to_string();
        changed.status = ProcessStatus::InProgress;
        changed.updated_at = 2000;

        assert!(repo.update(&changed).expect("update"));

        let fetched = repo.get("p1").expect("get").unwrap();
        assert_eq!(fetched.name, "Updated");
        assert_eq!(fetched.description, "new desc");
        assert_eq!(fetched.status, ProcessStatus::InProgress);
        assert_eq!(fetched.updated_at, 2000);
    }

    #[test]
    fn update_does_not_change_id_project_id_or_created_at() {
        let (_dir, repo, projects) = repos_in_temp_dir();
        projects.create(&sample_project("proj1")).unwrap();
        projects.create(&sample_project("proj2")).unwrap();
        let process = sample_process("p1", "proj1", "Original", 1000);
        repo.create(&process).unwrap();

        // Even if a caller (incorrectly) set a different project_id on the
        // struct passed to `update`, the SQL never writes project_id — a
        // process can't be reassigned to another project this way.
        let mut changed = process.clone();
        changed.project_id = "proj2".to_string();
        changed.name = "Updated".to_string();
        changed.updated_at = 2000;
        repo.update(&changed).expect("update");

        let fetched = repo.get("p1").expect("get").unwrap();
        assert_eq!(fetched.project_id, "proj1", "project_id must not change on update");
        assert_eq!(fetched.created_at, 1000, "created_at must not change on update");
    }

    #[test]
    fn update_missing_process_returns_false() {
        let (_dir, repo, projects) = repos_in_temp_dir();
        projects.create(&sample_project("proj1")).unwrap();
        let ghost = sample_process("does-not-exist", "proj1", "Ghost", 1000);
        assert!(!repo.update(&ghost).expect("update"));
    }

    #[test]
    fn delete_removes_the_process_and_reports_true() {
        let (_dir, repo, projects) = repos_in_temp_dir();
        projects.create(&sample_project("proj1")).unwrap();
        repo.create(&sample_process("p1", "proj1", "Process", 1000)).unwrap();

        assert!(repo.delete("p1").expect("delete"));
        assert!(repo.get("p1").expect("get").is_none());
    }

    #[test]
    fn delete_missing_process_reports_false() {
        let (_dir, repo, _projects) = repos_in_temp_dir();
        assert!(!repo.delete("does-not-exist").expect("delete"));
    }

    #[test]
    fn deleting_a_project_cascades_to_its_processes() {
        let (_dir, repo, projects) = repos_in_temp_dir();
        projects.create(&sample_project("proj1")).unwrap();
        repo.create(&sample_process("p1", "proj1", "Process A", 1000)).unwrap();
        repo.create(&sample_process("p2", "proj1", "Process B", 2000)).unwrap();

        // The database relationship (ON DELETE CASCADE), not application
        // code, owns this behavior — ProjectRepository::delete has no
        // knowledge of processes at all.
        assert!(projects.delete("proj1").expect("delete project"));

        assert!(repo.get("p1").expect("get").is_none());
        assert!(repo.get("p2").expect("get").is_none());
    }

    #[test]
    fn process_survives_reopening_the_database() {
        let dir = tempfile::tempdir().expect("temp dir");
        let process = sample_process("p1", "proj1", "Persisted process", 1000);

        {
            let db = DbService::init(dir.path()).expect("db init (first run)");
            SqliteProjectRepository::new(db.pool())
                .create(&sample_project("proj1"))
                .unwrap();
            SqliteProcessRepository::new(db.pool()).create(&process).expect("create");
        } // `db` dropped — simulates closing the app.

        {
            let db = DbService::init(dir.path()).expect("db init (second run)");
            let repo = SqliteProcessRepository::new(db.pool());
            let fetched = repo
                .get("p1")
                .expect("get")
                .expect("process should still exist after reopening the database");
            assert_eq!(fetched.name, process.name);
        } // simulates relaunching the app against the same data directory.
    }
}
