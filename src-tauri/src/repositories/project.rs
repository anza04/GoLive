//! `ProjectRepository`: the persistence boundary for the Project domain
//! (docs/architecture.md, "Persistence boundary"). All SQL for projects
//! lives here — nothing above this layer (service, commands, frontend)
//! knows or needs to know it's SQLite.

use crate::db::DbPool;
use crate::errors::AppError;
use crate::models::project::Project;
use rusqlite::{params, Row};

pub trait ProjectRepository: Send + Sync {
    fn create(&self, project: &Project) -> Result<(), AppError>;
    /// Ordered `updated_at DESC` — most recently touched project first.
    fn list(&self) -> Result<Vec<Project>, AppError>;
    fn get(&self, id: &str) -> Result<Option<Project>, AppError>;
    /// Writes `name`, `description`, and `updated_at` from `project` to
    /// the row matching `project.id`. `id` and `created_at` are not part
    /// of the `SET` clause — structurally impossible to change through
    /// this method, not just a convention callers are trusted to follow.
    /// Returns whether a row was actually updated (`false` if `id` didn't
    /// exist), so the caller can distinguish "updated" from "not found".
    fn update(&self, project: &Project) -> Result<bool, AppError>;
    /// Returns whether a row was actually deleted (`false` if `id` didn't
    /// exist), so the caller can distinguish "deleted" from "not found".
    fn delete(&self, id: &str) -> Result<bool, AppError>;
}

pub struct SqliteProjectRepository {
    pool: DbPool,
}

impl SqliteProjectRepository {
    pub fn new(pool: DbPool) -> Self {
        Self { pool }
    }
}

const SELECT_COLUMNS: &str = "id, name, description, created_at, updated_at";

fn row_to_project(row: &Row) -> rusqlite::Result<Project> {
    Ok(Project {
        id: row.get(0)?,
        name: row.get(1)?,
        description: row.get(2)?,
        created_at: row.get(3)?,
        updated_at: row.get(4)?,
    })
}

impl ProjectRepository for SqliteProjectRepository {
    fn create(&self, project: &Project) -> Result<(), AppError> {
        let conn = self.pool.get()?;
        conn.execute(
            "INSERT INTO projects (id, name, description, created_at, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5)",
            params![
                project.id,
                project.name,
                project.description,
                project.created_at,
                project.updated_at,
            ],
        )?;
        Ok(())
    }

    fn list(&self) -> Result<Vec<Project>, AppError> {
        let conn = self.pool.get()?;
        let mut stmt = conn.prepare(&format!(
            "SELECT {SELECT_COLUMNS} FROM projects ORDER BY updated_at DESC"
        ))?;
        let rows = stmt.query_map([], row_to_project)?;
        let mut projects = Vec::new();
        for row in rows {
            projects.push(row?);
        }
        Ok(projects)
    }

    fn get(&self, id: &str) -> Result<Option<Project>, AppError> {
        let conn = self.pool.get()?;
        let mut stmt = conn.prepare(&format!("SELECT {SELECT_COLUMNS} FROM projects WHERE id = ?1"))?;
        let mut rows = stmt.query(params![id])?;
        match rows.next()? {
            Some(row) => Ok(Some(row_to_project(row)?)),
            None => Ok(None),
        }
    }

    fn update(&self, project: &Project) -> Result<bool, AppError> {
        let conn = self.pool.get()?;
        let affected = conn.execute(
            "UPDATE projects SET name = ?1, description = ?2, updated_at = ?3 WHERE id = ?4",
            params![
                project.name,
                project.description,
                project.updated_at,
                project.id,
            ],
        )?;
        Ok(affected > 0)
    }

    fn delete(&self, id: &str) -> Result<bool, AppError> {
        let conn = self.pool.get()?;
        let affected = conn.execute("DELETE FROM projects WHERE id = ?1", params![id])?;
        Ok(affected > 0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::DbService;

    fn repo_in_temp_dir() -> (tempfile::TempDir, SqliteProjectRepository) {
        let dir = tempfile::tempdir().expect("temp dir");
        let db = DbService::init(dir.path()).expect("db init");
        (dir, SqliteProjectRepository::new(db.pool()))
    }

    fn sample(id: &str, name: &str, updated_at: i64) -> Project {
        Project {
            id: id.to_string(),
            name: name.to_string(),
            description: "desc".to_string(),
            created_at: updated_at,
            updated_at,
        }
    }

    #[test]
    fn create_then_get_round_trips() {
        let (_dir, repo) = repo_in_temp_dir();
        let project = sample("p1", "Migration ERP 2026", 1000);
        repo.create(&project).expect("create");

        let fetched = repo.get("p1").expect("get").expect("project should exist");
        assert_eq!(fetched.id, project.id);
        assert_eq!(fetched.name, project.name);
        assert_eq!(fetched.description, project.description);
        assert_eq!(fetched.created_at, project.created_at);
        assert_eq!(fetched.updated_at, project.updated_at);
    }

    #[test]
    fn get_missing_project_returns_none() {
        let (_dir, repo) = repo_in_temp_dir();
        assert!(repo.get("does-not-exist").expect("get").is_none());
    }

    #[test]
    fn list_orders_by_updated_at_descending() {
        let (_dir, repo) = repo_in_temp_dir();
        repo.create(&sample("older", "Older project", 1000)).unwrap();
        repo.create(&sample("newer", "Newer project", 2000)).unwrap();

        let listed = repo.list().expect("list");
        let ids: Vec<&str> = listed.iter().map(|p| p.id.as_str()).collect();
        assert_eq!(ids, vec!["newer", "older"]);
    }

    #[test]
    fn update_changes_name_and_description() {
        let (_dir, repo) = repo_in_temp_dir();
        let mut project = sample("p1", "Original name", 1000);
        project.description = "original description".to_string();
        repo.create(&project).unwrap();

        let mut changed = project.clone();
        changed.name = "New name".to_string();
        changed.description = "new description".to_string();
        changed.updated_at = 2000;

        let updated = repo.update(&changed).expect("update");
        assert!(updated);

        let fetched = repo.get("p1").expect("get").expect("project should exist");
        assert_eq!(fetched.name, "New name");
        assert_eq!(fetched.description, "new description");
        assert_eq!(fetched.updated_at, 2000);
    }

    #[test]
    fn update_does_not_change_id_or_created_at() {
        let (_dir, repo) = repo_in_temp_dir();
        let project = sample("p1", "Original name", 1000);
        repo.create(&project).unwrap();

        // Even if a caller (incorrectly) put different id/created_at values
        // on the struct passed to `update`, the SQL only ever targets the
        // row matching `project.id` and never writes `created_at` — the id
        // used to look up the row is what determines identity here.
        let mut changed = project.clone();
        changed.name = "New name".to_string();
        changed.updated_at = 2000;
        repo.update(&changed).expect("update");

        let fetched = repo.get("p1").expect("get").expect("project should exist");
        assert_eq!(fetched.id, "p1");
        assert_eq!(fetched.created_at, 1000, "created_at must not change on update");
    }

    #[test]
    fn update_missing_project_returns_false() {
        let (_dir, repo) = repo_in_temp_dir();
        let ghost = sample("does-not-exist", "Ghost", 1000);
        let updated = repo.update(&ghost).expect("update");
        assert!(!updated);
    }

    #[test]
    fn updating_a_project_moves_it_to_the_top_of_the_list() {
        let (_dir, repo) = repo_in_temp_dir();
        repo.create(&sample("older", "Older project", 1000)).unwrap();
        repo.create(&sample("newer", "Newer project", 2000)).unwrap();

        // Touch the older project with a newer `updated_at`.
        let mut touched = sample("older", "Older project", 3000);
        touched.description = "desc".to_string();
        repo.update(&touched).expect("update");

        let listed = repo.list().expect("list");
        let ids: Vec<&str> = listed.iter().map(|p| p.id.as_str()).collect();
        assert_eq!(ids, vec!["older", "newer"], "the just-updated project should now be first");
    }

    #[test]
    fn delete_removes_the_project_and_reports_true() {
        let (_dir, repo) = repo_in_temp_dir();
        repo.create(&sample("p1", "Project", 1000)).unwrap();

        let deleted = repo.delete("p1").expect("delete");
        assert!(deleted);
        assert!(repo.get("p1").expect("get").is_none());
    }

    #[test]
    fn delete_missing_project_reports_false() {
        let (_dir, repo) = repo_in_temp_dir();
        let deleted = repo.delete("does-not-exist").expect("delete");
        assert!(!deleted);
    }

    #[test]
    fn project_survives_reopening_the_database() {
        let dir = tempfile::tempdir().expect("temp dir");
        let project = sample("p1", "Persisted project", 1000);

        {
            let db = DbService::init(dir.path()).expect("db init (first run)");
            let repo = SqliteProjectRepository::new(db.pool());
            repo.create(&project).expect("create");
        } // `db` dropped — simulates closing the app.

        {
            let db = DbService::init(dir.path()).expect("db init (second run)");
            let repo = SqliteProjectRepository::new(db.pool());
            let fetched = repo
                .get("p1")
                .expect("get")
                .expect("project should still exist after reopening the database");
            assert_eq!(fetched.name, project.name);
        } // simulates relaunching the app against the same data directory.
    }
}
