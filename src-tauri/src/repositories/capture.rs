//! `CaptureRepository`: the persistence boundary for the Capture domain
//! (docs/architecture.md, "Persistence boundary"). All SQL for captures
//! lives here. No business rules — process-existence validation lives in
//! `CaptureService`, not here.

use crate::db::DbPool;
use crate::errors::AppError;
use crate::models::capture::Capture;
use rusqlite::{params, Row};

pub trait CaptureRepository: Send + Sync {
    fn create(&self, capture: &Capture) -> Result<(), AppError>;
    /// Ordered `updated_at DESC` — most recently touched capture first.
    /// Only captures belonging to `process_id` are returned.
    fn list_by_process(&self, process_id: &str) -> Result<Vec<Capture>, AppError>;
    fn get(&self, id: &str) -> Result<Option<Capture>, AppError>;
    /// Writes `type`, `title`, `description`, and `updated_at`. `id`,
    /// `process_id`, and `created_at` are not part of the `SET` clause —
    /// structurally impossible to change through this method. A Capture
    /// can never be moved to a different process this way.
    /// Returns whether a row was actually updated (`false` if `id` didn't
    /// exist), so the caller can distinguish "updated" from "not found".
    fn update(&self, capture: &Capture) -> Result<bool, AppError>;
    /// Returns whether a row was actually deleted (`false` if `id` didn't
    /// exist).
    fn delete(&self, id: &str) -> Result<bool, AppError>;
    /// Every Capture id currently in the database, across every process.
    /// Used only by `CaptureService::reconcile_media` (TASK-009) to find
    /// orphaned media files left behind by a Process/Project cascade
    /// delete this repository never directly participated in (see
    /// docs/architecture.md, "Cascade media cleanup").
    fn list_all_ids(&self) -> Result<Vec<String>, AppError>;
}

pub struct SqliteCaptureRepository {
    pool: DbPool,
}

impl SqliteCaptureRepository {
    pub fn new(pool: DbPool) -> Self {
        Self { pool }
    }
}

const SELECT_COLUMNS: &str = "id, process_id, type, title, description, created_at, updated_at";

fn row_to_capture(row: &Row) -> rusqlite::Result<Capture> {
    Ok(Capture {
        id: row.get(0)?,
        process_id: row.get(1)?,
        capture_type: row.get(2)?,
        title: row.get(3)?,
        description: row.get(4)?,
        created_at: row.get(5)?,
        updated_at: row.get(6)?,
    })
}

impl CaptureRepository for SqliteCaptureRepository {
    fn create(&self, capture: &Capture) -> Result<(), AppError> {
        let conn = self.pool.get()?;
        conn.execute(
            "INSERT INTO captures (id, process_id, type, title, description, created_at, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
            params![
                capture.id,
                capture.process_id,
                capture.capture_type,
                capture.title,
                capture.description,
                capture.created_at,
                capture.updated_at,
            ],
        )?;
        Ok(())
    }

    fn list_by_process(&self, process_id: &str) -> Result<Vec<Capture>, AppError> {
        let conn = self.pool.get()?;
        let mut stmt = conn.prepare(&format!(
            "SELECT {SELECT_COLUMNS} FROM captures WHERE process_id = ?1 ORDER BY updated_at DESC"
        ))?;
        let rows = stmt.query_map(params![process_id], row_to_capture)?;
        let mut captures = Vec::new();
        for row in rows {
            captures.push(row?);
        }
        Ok(captures)
    }

    fn get(&self, id: &str) -> Result<Option<Capture>, AppError> {
        let conn = self.pool.get()?;
        let mut stmt = conn.prepare(&format!("SELECT {SELECT_COLUMNS} FROM captures WHERE id = ?1"))?;
        let mut rows = stmt.query(params![id])?;
        match rows.next()? {
            Some(row) => Ok(Some(row_to_capture(row)?)),
            None => Ok(None),
        }
    }

    fn update(&self, capture: &Capture) -> Result<bool, AppError> {
        let conn = self.pool.get()?;
        let affected = conn.execute(
            "UPDATE captures SET type = ?1, title = ?2, description = ?3, updated_at = ?4 WHERE id = ?5",
            params![
                capture.capture_type,
                capture.title,
                capture.description,
                capture.updated_at,
                capture.id,
            ],
        )?;
        Ok(affected > 0)
    }

    fn delete(&self, id: &str) -> Result<bool, AppError> {
        let conn = self.pool.get()?;
        let affected = conn.execute("DELETE FROM captures WHERE id = ?1", params![id])?;
        Ok(affected > 0)
    }

    fn list_all_ids(&self) -> Result<Vec<String>, AppError> {
        let conn = self.pool.get()?;
        let mut stmt = conn.prepare("SELECT id FROM captures")?;
        let rows = stmt.query_map([], |row| row.get::<_, String>(0))?;
        let mut ids = Vec::new();
        for row in rows {
            ids.push(row?);
        }
        Ok(ids)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::DbService;
    use crate::models::capture::CaptureType;
    use crate::models::process::{Process, ProcessStatus};
    use crate::models::project::Project;
    use crate::repositories::process::{ProcessRepository, SqliteProcessRepository};
    use crate::repositories::project::{ProjectRepository, SqliteProjectRepository};

    fn repos_in_temp_dir() -> (
        tempfile::TempDir,
        SqliteCaptureRepository,
        SqliteProcessRepository,
        SqliteProjectRepository,
    ) {
        let dir = tempfile::tempdir().expect("temp dir");
        let db = DbService::init(dir.path()).expect("db init");
        (
            dir,
            SqliteCaptureRepository::new(db.pool()),
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

    fn sample_process(id: &str, project_id: &str) -> Process {
        Process {
            id: id.to_string(),
            project_id: project_id.to_string(),
            name: "Sample process".to_string(),
            description: String::new(),
            status: ProcessStatus::Draft,
            created_at: 1,
            updated_at: 1,
        }
    }

    fn sample_capture(id: &str, process_id: &str, title: &str, updated_at: i64) -> Capture {
        Capture {
            id: id.to_string(),
            process_id: process_id.to_string(),
            capture_type: CaptureType::Screenshot,
            title: title.to_string(),
            description: "desc".to_string(),
            created_at: updated_at,
            updated_at,
        }
    }

    #[test]
    fn create_then_get_round_trips() {
        let (_dir, repo, processes, projects) = repos_in_temp_dir();
        projects.create(&sample_project("proj1")).unwrap();
        processes.create(&sample_process("p1", "proj1")).unwrap();

        let capture = sample_capture("c1", "p1", "First screenshot", 1000);
        repo.create(&capture).expect("create");

        let fetched = repo.get("c1").expect("get").expect("capture should exist");
        assert_eq!(fetched.id, capture.id);
        assert_eq!(fetched.process_id, "p1");
        assert_eq!(fetched.title, capture.title);
        assert_eq!(fetched.description, capture.description);
        assert_eq!(fetched.capture_type, CaptureType::Screenshot);
    }

    #[test]
    fn get_missing_capture_returns_none() {
        let (_dir, repo, _processes, _projects) = repos_in_temp_dir();
        assert!(repo.get("does-not-exist").expect("get").is_none());
    }

    #[test]
    fn list_by_process_returns_only_matching_captures() {
        let (_dir, repo, processes, projects) = repos_in_temp_dir();
        projects.create(&sample_project("proj1")).unwrap();
        processes.create(&sample_process("p1", "proj1")).unwrap();
        processes.create(&sample_process("p2", "proj1")).unwrap();
        repo.create(&sample_capture("c1", "p1", "Capture A", 1000)).unwrap();
        repo.create(&sample_capture("c2", "p2", "Capture B", 2000)).unwrap();

        let listed = repo.list_by_process("p1").expect("list");
        assert_eq!(listed.len(), 1);
        assert_eq!(listed[0].id, "c1");
    }

    #[test]
    fn list_by_process_orders_by_updated_at_descending() {
        let (_dir, repo, processes, projects) = repos_in_temp_dir();
        projects.create(&sample_project("proj1")).unwrap();
        processes.create(&sample_process("p1", "proj1")).unwrap();
        repo.create(&sample_capture("older", "p1", "Older", 1000)).unwrap();
        repo.create(&sample_capture("newer", "p1", "Newer", 2000)).unwrap();

        let listed = repo.list_by_process("p1").expect("list");
        let ids: Vec<&str> = listed.iter().map(|c| c.id.as_str()).collect();
        assert_eq!(ids, vec!["newer", "older"]);
    }

    #[test]
    fn update_changes_type_title_and_description() {
        let (_dir, repo, processes, projects) = repos_in_temp_dir();
        projects.create(&sample_project("proj1")).unwrap();
        processes.create(&sample_process("p1", "proj1")).unwrap();
        let capture = sample_capture("c1", "p1", "Original", 1000);
        repo.create(&capture).unwrap();

        let mut changed = capture.clone();
        changed.capture_type = CaptureType::Note;
        changed.title = "Updated".to_string();
        changed.description = "new desc".to_string();
        changed.updated_at = 2000;

        assert!(repo.update(&changed).expect("update"));

        let fetched = repo.get("c1").expect("get").unwrap();
        assert_eq!(fetched.capture_type, CaptureType::Note);
        assert_eq!(fetched.title, "Updated");
        assert_eq!(fetched.description, "new desc");
        assert_eq!(fetched.updated_at, 2000);
    }

    #[test]
    fn update_does_not_change_id_process_id_or_created_at() {
        let (_dir, repo, processes, projects) = repos_in_temp_dir();
        projects.create(&sample_project("proj1")).unwrap();
        processes.create(&sample_process("p1", "proj1")).unwrap();
        processes.create(&sample_process("p2", "proj1")).unwrap();
        let capture = sample_capture("c1", "p1", "Original", 1000);
        repo.create(&capture).unwrap();

        // Even if a caller (incorrectly) set a different process_id on the
        // struct passed to `update`, the SQL never writes process_id — a
        // capture can't be reassigned to another process this way.
        let mut changed = capture.clone();
        changed.process_id = "p2".to_string();
        changed.title = "Updated".to_string();
        changed.updated_at = 2000;
        repo.update(&changed).expect("update");

        let fetched = repo.get("c1").expect("get").unwrap();
        assert_eq!(fetched.process_id, "p1", "process_id must not change on update");
        assert_eq!(fetched.created_at, 1000, "created_at must not change on update");
        assert_eq!(fetched.id, "c1", "id must not change on update");
    }

    #[test]
    fn update_missing_capture_returns_false() {
        let (_dir, repo, processes, projects) = repos_in_temp_dir();
        projects.create(&sample_project("proj1")).unwrap();
        processes.create(&sample_process("p1", "proj1")).unwrap();
        let ghost = sample_capture("does-not-exist", "p1", "Ghost", 1000);
        assert!(!repo.update(&ghost).expect("update"));
    }

    #[test]
    fn delete_removes_the_capture_and_reports_true() {
        let (_dir, repo, processes, projects) = repos_in_temp_dir();
        projects.create(&sample_project("proj1")).unwrap();
        processes.create(&sample_process("p1", "proj1")).unwrap();
        repo.create(&sample_capture("c1", "p1", "Capture", 1000)).unwrap();

        assert!(repo.delete("c1").expect("delete"));
        assert!(repo.get("c1").expect("get").is_none());
    }

    #[test]
    fn delete_missing_capture_reports_false() {
        let (_dir, repo, _processes, _projects) = repos_in_temp_dir();
        assert!(!repo.delete("does-not-exist").expect("delete"));
    }

    #[test]
    fn deleting_a_process_cascades_to_its_captures() {
        let (_dir, repo, processes, projects) = repos_in_temp_dir();
        projects.create(&sample_project("proj1")).unwrap();
        processes.create(&sample_process("p1", "proj1")).unwrap();
        repo.create(&sample_capture("c1", "p1", "Capture A", 1000)).unwrap();
        repo.create(&sample_capture("c2", "p1", "Capture B", 2000)).unwrap();

        // The database relationship (ON DELETE CASCADE), not application
        // code, owns this behavior — ProcessRepository::delete has no
        // knowledge of captures at all.
        assert!(processes.delete("p1").expect("delete process"));

        assert!(repo.get("c1").expect("get").is_none());
        assert!(repo.get("c2").expect("get").is_none());
    }

    #[test]
    fn deleting_a_project_cascades_through_processes_to_captures() {
        let (_dir, repo, processes, projects) = repos_in_temp_dir();
        projects.create(&sample_project("proj1")).unwrap();
        processes.create(&sample_process("p1", "proj1")).unwrap();
        repo.create(&sample_capture("c1", "p1", "Capture A", 1000)).unwrap();

        // Two chained ON DELETE CASCADE relationships, not an application-
        // side cascade loop: deleting the project deletes its processes,
        // which deletes their captures.
        assert!(projects.delete("proj1").expect("delete project"));

        assert!(processes.get("p1").expect("get").is_none());
        assert!(repo.get("c1").expect("get").is_none());
    }

    #[test]
    fn list_all_ids_returns_every_capture_across_processes() {
        let (_dir, repo, processes, projects) = repos_in_temp_dir();
        projects.create(&sample_project("proj1")).unwrap();
        processes.create(&sample_process("p1", "proj1")).unwrap();
        processes.create(&sample_process("p2", "proj1")).unwrap();
        repo.create(&sample_capture("c1", "p1", "Capture A", 1000)).unwrap();
        repo.create(&sample_capture("c2", "p2", "Capture B", 2000)).unwrap();

        let mut ids = repo.list_all_ids().expect("list_all_ids");
        ids.sort();
        assert_eq!(ids, vec!["c1".to_string(), "c2".to_string()]);
    }

    #[test]
    fn capture_survives_reopening_the_database() {
        let dir = tempfile::tempdir().expect("temp dir");
        let capture = sample_capture("c1", "p1", "Persisted capture", 1000);

        {
            let db = DbService::init(dir.path()).expect("db init (first run)");
            SqliteProjectRepository::new(db.pool())
                .create(&sample_project("proj1"))
                .unwrap();
            SqliteProcessRepository::new(db.pool())
                .create(&sample_process("p1", "proj1"))
                .unwrap();
            SqliteCaptureRepository::new(db.pool()).create(&capture).expect("create");
        } // `db` dropped — simulates closing the app.

        {
            let db = DbService::init(dir.path()).expect("db init (second run)");
            let repo = SqliteCaptureRepository::new(db.pool());
            let fetched = repo
                .get("c1")
                .expect("get")
                .expect("capture should still exist after reopening the database");
            assert_eq!(fetched.title, capture.title);
        } // simulates relaunching the app against the same data directory.
    }
}
