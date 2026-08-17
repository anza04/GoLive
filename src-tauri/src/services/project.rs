//! `ProjectService`: the Project domain's business logic — validation, ID
//! and timestamp generation — sitting between the thin Tauri commands and
//! `ProjectRepository`. See docs/architecture.md ("Business logic
//! boundary"): this is the first real occupant of that boundary.
//!
//! Deliberately holds a `Box<dyn ProjectRepository>` rather than a
//! concrete `SqliteProjectRepository` — commands depend on this service,
//! this service depends only on the trait, and the SQLite implementation
//! is the only thing a future remote/cloud repository would replace.

use crate::errors::AppError;
use crate::models::project::Project;
use crate::repositories::project::ProjectRepository;
use uuid::Uuid;

/// Project content language is configurable per project (see the product
/// brief), so lengths are counted in Unicode scalar values, not bytes —
/// a 200-character Italian name shouldn't be rejected earlier than a
/// 200-character English one just because it has more accented letters.
const MAX_NAME_LENGTH: usize = 200;
const MAX_DESCRIPTION_LENGTH: usize = 5000;

pub struct ProjectService {
    repository: Box<dyn ProjectRepository>,
}

impl ProjectService {
    pub fn new(repository: Box<dyn ProjectRepository>) -> Self {
        Self { repository }
    }

    /// Trims and validates `name`/`description`, generates the id and
    /// timestamps (never trusting frontend-supplied values for any of
    /// these), persists, and returns the created project.
    pub fn create(&self, name: &str, description: Option<&str>) -> Result<Project, AppError> {
        let name = validate_name(name)?;
        let description = validate_description(description.unwrap_or(""))?;
        let now = now_ms();

        let project = Project {
            id: Uuid::new_v4().to_string(),
            name,
            description,
            created_at: now,
            updated_at: now,
        };

        self.repository.create(&project)?;
        Ok(project)
    }

    pub fn list(&self) -> Result<Vec<Project>, AppError> {
        self.repository.list()
    }

    /// Trims and validates `name`/`description` (same rules as `create`),
    /// verifies the project exists, regenerates `updated_at`, and persists
    /// the change. `id` and `created_at` are taken from the existing
    /// record, never from the caller — the frontend cannot influence
    /// either.
    pub fn update(&self, id: &str, name: &str, description: Option<&str>) -> Result<Project, AppError> {
        let name = validate_name(name)?;
        let description = validate_description(description.unwrap_or(""))?;

        let existing = self.get(id)?;

        let project = Project {
            id: existing.id,
            name,
            description,
            created_at: existing.created_at,
            updated_at: now_ms(),
        };

        if self.repository.update(&project)? {
            Ok(project)
        } else {
            // The row existed a moment ago (`self.get` above) but is gone
            // now — treat the same as "not found" rather than silently
            // reporting success for a write that didn't land anywhere.
            Err(AppError::NotFound)
        }
    }

    pub fn get(&self, id: &str) -> Result<Project, AppError> {
        self.repository.get(id)?.ok_or(AppError::NotFound)
    }

    pub fn delete(&self, id: &str) -> Result<(), AppError> {
        if self.repository.delete(id)? {
            Ok(())
        } else {
            Err(AppError::NotFound)
        }
    }
}

fn validate_name(raw: &str) -> Result<String, AppError> {
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return Err(AppError::Validation("Project name is required.".to_string()));
    }
    if trimmed.chars().count() > MAX_NAME_LENGTH {
        return Err(AppError::Validation(format!(
            "Project name must be {MAX_NAME_LENGTH} characters or fewer."
        )));
    }
    Ok(trimmed.to_string())
}

fn validate_description(raw: &str) -> Result<String, AppError> {
    let trimmed = raw.trim();
    if trimmed.chars().count() > MAX_DESCRIPTION_LENGTH {
        return Err(AppError::Validation(format!(
            "Description must be {MAX_DESCRIPTION_LENGTH} characters or fewer."
        )));
    }
    Ok(trimmed.to_string())
}

fn now_ms() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis() as i64)
        .unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::DbService;
    use crate::repositories::project::SqliteProjectRepository;

    fn service_in_temp_dir() -> (tempfile::TempDir, ProjectService) {
        let dir = tempfile::tempdir().expect("temp dir");
        let db = DbService::init(dir.path()).expect("db init");
        let repo = SqliteProjectRepository::new(db.pool());
        (dir, ProjectService::new(Box::new(repo)))
    }

    #[test]
    fn create_generates_id_and_timestamps() {
        let (_dir, service) = service_in_temp_dir();
        let project = service.create("Migration ERP 2026", None).expect("create");

        assert!(!project.id.is_empty());
        assert!(uuid::Uuid::parse_str(&project.id).is_ok(), "id should be a UUID");
        assert_eq!(project.created_at, project.updated_at);
        assert!(project.created_at > 0);
    }

    #[test]
    fn create_trims_name_and_description() {
        let (_dir, service) = service_in_temp_dir();
        let project = service
            .create("  Migration ERP 2026  ", Some("  some notes  "))
            .expect("create");

        assert_eq!(project.name, "Migration ERP 2026");
        assert_eq!(project.description, "some notes");
    }

    #[test]
    fn create_rejects_empty_name() {
        let (_dir, service) = service_in_temp_dir();
        let result = service.create("", None);
        assert!(matches!(result, Err(AppError::Validation(_))));
    }

    #[test]
    fn create_rejects_whitespace_only_name() {
        let (_dir, service) = service_in_temp_dir();
        let result = service.create("    ", None);
        assert!(matches!(result, Err(AppError::Validation(_))));
    }

    #[test]
    fn create_rejects_name_over_the_length_limit() {
        let (_dir, service) = service_in_temp_dir();
        let too_long = "a".repeat(MAX_NAME_LENGTH + 1);
        let result = service.create(&too_long, None);
        assert!(matches!(result, Err(AppError::Validation(_))));
    }

    #[test]
    fn create_rejects_description_over_the_length_limit() {
        let (_dir, service) = service_in_temp_dir();
        let too_long = "a".repeat(MAX_DESCRIPTION_LENGTH + 1);
        let result = service.create("Valid name", Some(&too_long));
        assert!(matches!(result, Err(AppError::Validation(_))));
    }

    #[test]
    fn create_allows_missing_or_empty_description() {
        let (_dir, service) = service_in_temp_dir();
        assert!(service.create("Project A", None).is_ok());
        assert!(service.create("Project B", Some("")).is_ok());
        assert!(service.create("Project C", Some("   ")).is_ok());
    }

    #[test]
    fn list_returns_created_projects() {
        let (_dir, service) = service_in_temp_dir();
        service.create("Project A", None).unwrap();
        service.create("Project B", None).unwrap();

        let listed = service.list().expect("list");
        assert_eq!(listed.len(), 2);
    }

    #[test]
    fn get_missing_project_returns_not_found() {
        let (_dir, service) = service_in_temp_dir();
        let result = service.get("does-not-exist");
        assert!(matches!(result, Err(AppError::NotFound)));
    }

    #[test]
    fn get_returns_created_project() {
        let (_dir, service) = service_in_temp_dir();
        let created = service.create("Project A", None).unwrap();
        let fetched = service.get(&created.id).expect("get");
        assert_eq!(fetched.id, created.id);
    }

    #[test]
    fn update_succeeds_and_trims_name_and_description() {
        let (_dir, service) = service_in_temp_dir();
        let created = service.create("Original", Some("original desc")).unwrap();

        let updated = service
            .update(&created.id, "  Updated name  ", Some("  updated desc  "))
            .expect("update");

        assert_eq!(updated.name, "Updated name");
        assert_eq!(updated.description, "updated desc");
    }

    #[test]
    fn update_regenerates_updated_at_and_preserves_created_at() {
        let (_dir, service) = service_in_temp_dir();
        let created = service.create("Original", None).unwrap();

        // Guarantee a different millisecond so `updated_at` is provably
        // regenerated, not coincidentally identical to `created_at`.
        std::thread::sleep(std::time::Duration::from_millis(5));

        let updated = service.update(&created.id, "Updated", None).expect("update");

        assert_eq!(updated.id, created.id, "id must not change");
        assert_eq!(updated.created_at, created.created_at, "created_at must not change");
        assert!(
            updated.updated_at > created.updated_at,
            "updated_at must be regenerated by the backend"
        );
    }

    #[test]
    fn update_rejects_empty_name() {
        let (_dir, service) = service_in_temp_dir();
        let created = service.create("Original", None).unwrap();
        let result = service.update(&created.id, "", None);
        assert!(matches!(result, Err(AppError::Validation(_))));
    }

    #[test]
    fn update_rejects_whitespace_only_name() {
        let (_dir, service) = service_in_temp_dir();
        let created = service.create("Original", None).unwrap();
        let result = service.update(&created.id, "   ", None);
        assert!(matches!(result, Err(AppError::Validation(_))));
    }

    #[test]
    fn update_rejects_name_over_the_length_limit() {
        let (_dir, service) = service_in_temp_dir();
        let created = service.create("Original", None).unwrap();
        let too_long = "a".repeat(MAX_NAME_LENGTH + 1);
        let result = service.update(&created.id, &too_long, None);
        assert!(matches!(result, Err(AppError::Validation(_))));
    }

    #[test]
    fn update_rejects_description_over_the_length_limit() {
        let (_dir, service) = service_in_temp_dir();
        let created = service.create("Original", None).unwrap();
        let too_long = "a".repeat(MAX_DESCRIPTION_LENGTH + 1);
        let result = service.update(&created.id, "Valid name", Some(&too_long));
        assert!(matches!(result, Err(AppError::Validation(_))));
    }

    #[test]
    fn update_missing_project_returns_not_found() {
        let (_dir, service) = service_in_temp_dir();
        let result = service.update("does-not-exist", "New name", None);
        assert!(matches!(result, Err(AppError::NotFound)));
    }

    #[test]
    fn delete_removes_the_project() {
        let (_dir, service) = service_in_temp_dir();
        let created = service.create("Project A", None).unwrap();

        service.delete(&created.id).expect("delete");
        assert!(matches!(service.get(&created.id), Err(AppError::NotFound)));
    }

    #[test]
    fn delete_missing_project_returns_not_found() {
        let (_dir, service) = service_in_temp_dir();
        let result = service.delete("does-not-exist");
        assert!(matches!(result, Err(AppError::NotFound)));
    }
}
