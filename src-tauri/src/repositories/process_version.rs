//! `ProcessVersionRepository`: the persistence boundary for the
//! ProcessVersion domain (TASK-018, editable since TASK-019). All SQL
//! for process versions lives here. No business rules — those live in
//! `services::process_draft` (docs/architecture.md §5).
//!
//! No `delete`: a ProcessVersion is never individually removed at this
//! stage (only cascaded away with its Process — see
//! `migrations/0005_process_versions.sql`). `update_content` exists
//! (added by TASK-019) specifically for a user editing and saving their
//! own draft — it is deliberately *not* how regeneration works: a new
//! generation always goes through `create`, inserting a brand new row,
//! never touching an existing one. See DECISIONS.md for the full
//! reasoning behind drawing that line here rather than treating every
//! content change the same way.

use crate::ai::ProcessDraft;
use crate::db::DbPool;
use crate::errors::AppError;
use crate::models::process_version::ProcessVersion;
use rusqlite::{params, Row};

pub trait ProcessVersionRepository: Send + Sync {
    fn create(&self, version: &ProcessVersion) -> Result<(), AppError>;
    /// Ordered `created_at DESC` — newest version first. Only versions
    /// belonging to `process_id` are returned.
    fn list_by_process(&self, process_id: &str) -> Result<Vec<ProcessVersion>, AppError>;
    /// The single newest version for `process_id`, or `None` if it has
    /// never been generated. A dedicated query rather than
    /// `list_by_process(...).first()` at a caller — each version's
    /// `content` can be a sizable JSON blob (a full draft with many
    /// steps), so fetching every version just to look at the newest one
    /// would be real, avoidable waste.
    fn get_latest_by_process(&self, process_id: &str) -> Result<Option<ProcessVersion>, AppError>;
    fn get(&self, id: &str) -> Result<Option<ProcessVersion>, AppError>;
    /// Writes `content` and `updated_at` only (TASK-019, user edits) —
    /// `id`/`process_id`/`created_at` are not part of the `SET` clause,
    /// same "structurally can't be changed through update" convention
    /// every other domain's `update` follows. Returns whether a row was
    /// actually updated (`false` if `id` didn't exist).
    fn update_content(&self, id: &str, content: &ProcessDraft, updated_at: i64) -> Result<bool, AppError>;
}

pub struct SqliteProcessVersionRepository {
    pool: DbPool,
}

impl SqliteProcessVersionRepository {
    pub fn new(pool: DbPool) -> Self {
        Self { pool }
    }
}

const SELECT_COLUMNS: &str = "id, process_id, content, created_at, updated_at";

/// Parses the `content` column's JSON text back into an `ai::ProcessDraft`.
/// A failure here (corrupt/foreign JSON somehow ending up in this
/// column) is a `Database` error, not `NotFound` or a panic — the row
/// exists, it's just unreadable, the same category of problem as any
/// other SQL-level read failure.
fn row_to_version(row: &Row) -> rusqlite::Result<ProcessVersion> {
    let content_json: String = row.get(2)?;
    let content: ProcessDraft = serde_json::from_str(&content_json).map_err(|err| {
        rusqlite::Error::FromSqlConversionFailure(2, rusqlite::types::Type::Text, Box::new(err))
    })?;
    Ok(ProcessVersion {
        id: row.get(0)?,
        process_id: row.get(1)?,
        content,
        created_at: row.get(3)?,
        updated_at: row.get(4)?,
    })
}

impl ProcessVersionRepository for SqliteProcessVersionRepository {
    fn create(&self, version: &ProcessVersion) -> Result<(), AppError> {
        let content_json = serde_json::to_string(&version.content).map_err(|err| {
            eprintln!("[golive] failed to serialize a ProcessVersion's content: {err}");
            AppError::Database
        })?;
        let conn = self.pool.get()?;
        conn.execute(
            "INSERT INTO process_versions (id, process_id, content, created_at, updated_at) VALUES (?1, ?2, ?3, ?4, ?5)",
            params![version.id, version.process_id, content_json, version.created_at, version.updated_at],
        )?;
        Ok(())
    }

    fn list_by_process(&self, process_id: &str) -> Result<Vec<ProcessVersion>, AppError> {
        let conn = self.pool.get()?;
        let mut stmt = conn.prepare(&format!(
            "SELECT {SELECT_COLUMNS} FROM process_versions WHERE process_id = ?1 ORDER BY created_at DESC"
        ))?;
        let rows = stmt.query_map(params![process_id], row_to_version)?;
        let mut versions = Vec::new();
        for row in rows {
            versions.push(row?);
        }
        Ok(versions)
    }

    fn get_latest_by_process(&self, process_id: &str) -> Result<Option<ProcessVersion>, AppError> {
        let conn = self.pool.get()?;
        let mut stmt = conn.prepare(&format!(
            "SELECT {SELECT_COLUMNS} FROM process_versions WHERE process_id = ?1 ORDER BY created_at DESC LIMIT 1"
        ))?;
        let mut rows = stmt.query(params![process_id])?;
        match rows.next()? {
            Some(row) => Ok(Some(row_to_version(row)?)),
            None => Ok(None),
        }
    }

    fn get(&self, id: &str) -> Result<Option<ProcessVersion>, AppError> {
        let conn = self.pool.get()?;
        let mut stmt = conn.prepare(&format!("SELECT {SELECT_COLUMNS} FROM process_versions WHERE id = ?1"))?;
        let mut rows = stmt.query(params![id])?;
        match rows.next()? {
            Some(row) => Ok(Some(row_to_version(row)?)),
            None => Ok(None),
        }
    }

    fn update_content(&self, id: &str, content: &ProcessDraft, updated_at: i64) -> Result<bool, AppError> {
        let content_json = serde_json::to_string(content).map_err(|err| {
            eprintln!("[golive] failed to serialize edited ProcessVersion content: {err}");
            AppError::Database
        })?;
        let conn = self.pool.get()?;
        let affected = conn.execute(
            "UPDATE process_versions SET content = ?1, updated_at = ?2 WHERE id = ?3",
            params![content_json, updated_at, id],
        )?;
        Ok(affected > 0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ai::ProcessDraftStep;
    use crate::db::DbService;
    use crate::models::process::{Process, ProcessStatus};
    use crate::models::project::Project;
    use crate::repositories::process::{ProcessRepository, SqliteProcessRepository};
    use crate::repositories::project::{ProjectRepository, SqliteProjectRepository};

    fn repos_in_temp_dir() -> (
        tempfile::TempDir,
        SqliteProcessVersionRepository,
        SqliteProcessRepository,
        SqliteProjectRepository,
    ) {
        let dir = tempfile::tempdir().expect("temp dir");
        let db = DbService::init(dir.path()).expect("db init");
        (
            dir,
            SqliteProcessVersionRepository::new(db.pool()),
            SqliteProcessRepository::new(db.pool()),
            SqliteProjectRepository::new(db.pool()),
        )
    }

    fn sample_project(id: &str) -> Project {
        Project { id: id.to_string(), name: "P".to_string(), description: String::new(), created_at: 1, updated_at: 1 }
    }

    fn sample_process(id: &str, project_id: &str) -> Process {
        Process {
            id: id.to_string(),
            project_id: project_id.to_string(),
            name: "Proc".to_string(),
            description: String::new(),
            status: ProcessStatus::Draft,
            created_at: 1,
            updated_at: 1,
        }
    }

    fn sample_draft(summary: &str) -> ProcessDraft {
        ProcessDraft {
            summary: summary.to_string(),
            steps: vec![ProcessDraftStep {
                title: "Step".to_string(),
                description: "Desc".to_string(),
                capture_ids: vec!["cap-1".to_string()],
            }],
        }
    }

    fn sample_version(id: &str, process_id: &str, summary: &str, created_at: i64) -> ProcessVersion {
        ProcessVersion {
            id: id.to_string(),
            process_id: process_id.to_string(),
            content: sample_draft(summary),
            created_at,
            updated_at: created_at,
        }
    }

    fn seed_process(processes: &SqliteProcessRepository, projects: &SqliteProjectRepository, process_id: &str) {
        projects.create(&sample_project("proj1")).unwrap();
        processes.create(&sample_process(process_id, "proj1")).unwrap();
    }

    #[test]
    fn create_then_get_round_trips_the_full_content() {
        let (_dir, repo, processes, projects) = repos_in_temp_dir();
        seed_process(&processes, &projects, "p1");

        let version = sample_version("v1", "p1", "A summary", 1000);
        repo.create(&version).expect("create");

        let fetched = repo.get("v1").expect("get").expect("version should exist");
        assert_eq!(fetched, version);
    }

    #[test]
    fn get_missing_version_returns_none() {
        let (_dir, repo, _processes, _projects) = repos_in_temp_dir();
        assert!(repo.get("does-not-exist").expect("get").is_none());
    }

    #[test]
    fn generating_twice_produces_two_retrievable_versions_neither_overwriting_the_other() {
        let (_dir, repo, processes, projects) = repos_in_temp_dir();
        seed_process(&processes, &projects, "p1");

        repo.create(&sample_version("v1", "p1", "First generation", 1000)).expect("create v1");
        repo.create(&sample_version("v2", "p1", "Second generation", 2000)).expect("create v2");

        let listed = repo.list_by_process("p1").expect("list");
        assert_eq!(listed.len(), 2);
        // Newest first.
        assert_eq!(listed[0].id, "v2");
        assert_eq!(listed[1].id, "v1");
        assert_eq!(repo.get("v1").expect("get v1").unwrap().content.summary, "First generation");
        assert_eq!(repo.get("v2").expect("get v2").unwrap().content.summary, "Second generation");
    }

    #[test]
    fn list_by_process_returns_only_matching_versions() {
        let (_dir, repo, processes, projects) = repos_in_temp_dir();
        projects.create(&sample_project("proj1")).unwrap();
        processes.create(&sample_process("p1", "proj1")).unwrap();
        processes.create(&sample_process("p2", "proj1")).unwrap();
        repo.create(&sample_version("v1", "p1", "For p1", 1000)).unwrap();
        repo.create(&sample_version("v2", "p2", "For p2", 1000)).unwrap();

        let listed = repo.list_by_process("p1").expect("list");
        assert_eq!(listed.len(), 1);
        assert_eq!(listed[0].id, "v1");
    }

    #[test]
    fn list_by_process_for_a_process_with_no_versions_is_empty() {
        let (_dir, repo, processes, projects) = repos_in_temp_dir();
        seed_process(&processes, &projects, "p1");
        assert!(repo.list_by_process("p1").expect("list").is_empty());
    }

    #[test]
    fn get_latest_by_process_returns_the_newest_version() {
        let (_dir, repo, processes, projects) = repos_in_temp_dir();
        seed_process(&processes, &projects, "p1");
        repo.create(&sample_version("v1", "p1", "Older", 1000)).unwrap();
        repo.create(&sample_version("v2", "p1", "Newer", 2000)).unwrap();

        let latest = repo.get_latest_by_process("p1").expect("get_latest").expect("should exist");
        assert_eq!(latest.id, "v2");
        assert_eq!(latest.content.summary, "Newer");
    }

    #[test]
    fn get_latest_by_process_for_a_process_with_no_versions_returns_none() {
        let (_dir, repo, processes, projects) = repos_in_temp_dir();
        seed_process(&processes, &projects, "p1");
        assert!(repo.get_latest_by_process("p1").expect("get_latest").is_none());
    }

    #[test]
    fn deleting_a_process_cascades_to_its_versions() {
        let (_dir, repo, processes, projects) = repos_in_temp_dir();
        seed_process(&processes, &projects, "p1");
        repo.create(&sample_version("v1", "p1", "Summary", 1000)).unwrap();

        assert!(processes.delete("p1").expect("delete process"));

        assert!(repo.get("v1").expect("get after cascade").is_none());
    }

    #[test]
    fn version_survives_reopening_the_database() {
        let dir = tempfile::tempdir().expect("temp dir");
        {
            let db = DbService::init(dir.path()).expect("db init");
            let projects = SqliteProjectRepository::new(db.pool());
            let processes = SqliteProcessRepository::new(db.pool());
            let repo = SqliteProcessVersionRepository::new(db.pool());
            seed_process(&processes, &projects, "p1");
            repo.create(&sample_version("v1", "p1", "Persisted", 1000)).unwrap();
        }
        let db = DbService::init(dir.path()).expect("db reopen");
        let repo = SqliteProcessVersionRepository::new(db.pool());
        let fetched = repo.get("v1").expect("get").expect("version should survive reopening");
        assert_eq!(fetched.content.summary, "Persisted");
    }

    #[test]
    fn update_content_changes_content_and_updated_at_but_not_created_at_id_or_process_id() {
        let (_dir, repo, processes, projects) = repos_in_temp_dir();
        seed_process(&processes, &projects, "p1");
        repo.create(&sample_version("v1", "p1", "Original", 1000)).unwrap();

        let edited = sample_draft("Edited by hand");
        let updated = repo.update_content("v1", &edited, 5000).expect("update_content");
        assert!(updated);

        let fetched = repo.get("v1").expect("get").unwrap();
        assert_eq!(fetched.content.summary, "Edited by hand");
        assert_eq!(fetched.updated_at, 5000);
        assert_eq!(fetched.created_at, 1000, "created_at must never change");
        assert_eq!(fetched.id, "v1");
        assert_eq!(fetched.process_id, "p1");
    }

    #[test]
    fn update_content_on_a_missing_version_returns_false() {
        let (_dir, repo, _processes, _projects) = repos_in_temp_dir();
        let updated = repo.update_content("does-not-exist", &sample_draft("x"), 1000).expect("update_content");
        assert!(!updated);
    }

    #[test]
    fn update_content_never_creates_a_second_row() {
        let (_dir, repo, processes, projects) = repos_in_temp_dir();
        seed_process(&processes, &projects, "p1");
        repo.create(&sample_version("v1", "p1", "Original", 1000)).unwrap();

        repo.update_content("v1", &sample_draft("Edited"), 2000).unwrap();
        repo.update_content("v1", &sample_draft("Edited again"), 3000).unwrap();

        let listed = repo.list_by_process("p1").expect("list");
        assert_eq!(listed.len(), 1, "editing must update the existing row, never insert a new one");
        assert_eq!(listed[0].content.summary, "Edited again");
    }
}
