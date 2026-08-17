//! `ProcessService`: the Process domain's business logic — validation, ID/
//! timestamp generation, project-existence enforcement, and status
//! parsing — sitting between the thin Tauri commands and
//! `ProcessRepository`. See docs/architecture.md ("Business logic
//! boundary").
//!
//! Holds both a `Box<dyn ProcessRepository>` and a
//! `Box<dyn ProjectRepository>`: creating a process needs to confirm its
//! parent project exists (see docs/architecture.md, "Project ownership")
//! before writing anything, which needs the project repository, not just
//! the process one.

use crate::errors::AppError;
use crate::models::process::{Process, ProcessStatus};
use crate::repositories::process::ProcessRepository;
use crate::repositories::project::ProjectRepository;
use uuid::Uuid;

/// Same limits and Unicode-scalar-value counting rationale as
/// `ProjectService` (see there) — kept as this service's own constants
/// rather than a shared import, consistent with each domain service being
/// self-contained.
const MAX_NAME_LENGTH: usize = 200;
const MAX_DESCRIPTION_LENGTH: usize = 5000;

pub struct ProcessService {
    repository: Box<dyn ProcessRepository>,
    projects: Box<dyn ProjectRepository>,
}

impl ProcessService {
    pub fn new(repository: Box<dyn ProcessRepository>, projects: Box<dyn ProjectRepository>) -> Self {
        Self { repository, projects }
    }

    /// Verifies `project_id` refers to a real project, trims/validates
    /// `name`/`description`, generates the id and timestamps, and sets
    /// status to `Draft` — the backend establishes the initial state, the
    /// frontend never specifies it on create.
    pub fn create(
        &self,
        project_id: &str,
        name: &str,
        description: Option<&str>,
    ) -> Result<Process, AppError> {
        let name = validate_name(name)?;
        let description = validate_description(description.unwrap_or(""))?;

        // Confirms the project exists before creating a process under it.
        // The `project_id` foreign key (ON DELETE CASCADE) would also
        // reject an orphaned insert, but checking here first gives a
        // clear AppError::NotFound instead of a generic database error.
        self.projects.get(project_id)?.ok_or(AppError::NotFound)?;

        let now = now_ms();
        let process = Process {
            id: Uuid::new_v4().to_string(),
            project_id: project_id.to_string(),
            name,
            description,
            status: ProcessStatus::Draft,
            created_at: now,
            updated_at: now,
        };

        self.repository.create(&process)?;
        Ok(process)
    }

    pub fn list_by_project(&self, project_id: &str) -> Result<Vec<Process>, AppError> {
        self.repository.list_by_project(project_id)
    }

    pub fn get(&self, id: &str) -> Result<Process, AppError> {
        self.repository.get(id)?.ok_or(AppError::NotFound)
    }

    /// Trims/validates `name`/`description`, parses `status` (rejecting
    /// anything other than the three defined values), verifies the
    /// process exists, and regenerates `updated_at`. `id`, `project_id`,
    /// and `created_at` are taken from the existing record — a process
    /// can never be moved to another project through update.
    pub fn update(
        &self,
        id: &str,
        name: &str,
        description: Option<&str>,
        status: &str,
    ) -> Result<Process, AppError> {
        let name = validate_name(name)?;
        let description = validate_description(description.unwrap_or(""))?;
        let status = ProcessStatus::parse(status)
            .ok_or_else(|| AppError::Validation("Invalid process status.".to_string()))?;

        let existing = self.get(id)?;

        let process = Process {
            id: existing.id,
            project_id: existing.project_id,
            name,
            description,
            status,
            created_at: existing.created_at,
            updated_at: now_ms(),
        };

        if self.repository.update(&process)? {
            Ok(process)
        } else {
            Err(AppError::NotFound)
        }
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
        return Err(AppError::Validation("Process name is required.".to_string()));
    }
    if trimmed.chars().count() > MAX_NAME_LENGTH {
        return Err(AppError::Validation(format!(
            "Process name must be {MAX_NAME_LENGTH} characters or fewer."
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
    use crate::models::project::Project;
    use crate::repositories::process::SqliteProcessRepository;
    use crate::repositories::project::SqliteProjectRepository;

    fn service_with_project() -> (tempfile::TempDir, ProcessService, String) {
        let dir = tempfile::tempdir().expect("temp dir");
        let db = DbService::init(dir.path()).expect("db init");

        let project = Project {
            id: "proj1".to_string(),
            name: "Sample project".to_string(),
            description: String::new(),
            created_at: 1,
            updated_at: 1,
        };
        SqliteProjectRepository::new(db.pool()).create(&project).unwrap();

        let service = ProcessService::new(
            Box::new(SqliteProcessRepository::new(db.pool())),
            Box::new(SqliteProjectRepository::new(db.pool())),
        );
        (dir, service, project.id)
    }

    #[test]
    fn create_generates_id_and_defaults_to_draft() {
        let (_dir, service, project_id) = service_with_project();
        let process = service
            .create(&project_id, "Create a sales order", None)
            .expect("create");

        assert!(!process.id.is_empty());
        assert!(uuid::Uuid::parse_str(&process.id).is_ok(), "id should be a UUID");
        assert_eq!(process.project_id, project_id);
        assert_eq!(process.status, ProcessStatus::Draft);
        assert_eq!(process.created_at, process.updated_at);
    }

    #[test]
    fn create_trims_name_and_description() {
        let (_dir, service, project_id) = service_with_project();
        let process = service
            .create(&project_id, "  Create a sales order  ", Some("  notes  "))
            .expect("create");

        assert_eq!(process.name, "Create a sales order");
        assert_eq!(process.description, "notes");
    }

    #[test]
    fn create_rejects_empty_name() {
        let (_dir, service, project_id) = service_with_project();
        assert!(matches!(
            service.create(&project_id, "", None),
            Err(AppError::Validation(_))
        ));
    }

    #[test]
    fn create_rejects_whitespace_only_name() {
        let (_dir, service, project_id) = service_with_project();
        assert!(matches!(
            service.create(&project_id, "   ", None),
            Err(AppError::Validation(_))
        ));
    }

    #[test]
    fn create_rejects_name_over_the_length_limit() {
        let (_dir, service, project_id) = service_with_project();
        let too_long = "a".repeat(MAX_NAME_LENGTH + 1);
        assert!(matches!(
            service.create(&project_id, &too_long, None),
            Err(AppError::Validation(_))
        ));
    }

    #[test]
    fn create_rejects_description_over_the_length_limit() {
        let (_dir, service, project_id) = service_with_project();
        let too_long = "a".repeat(MAX_DESCRIPTION_LENGTH + 1);
        assert!(matches!(
            service.create(&project_id, "Valid name", Some(&too_long)),
            Err(AppError::Validation(_))
        ));
    }

    #[test]
    fn create_rejects_missing_project() {
        let (_dir, service, _project_id) = service_with_project();
        let result = service.create("does-not-exist", "Valid name", None);
        assert!(matches!(result, Err(AppError::NotFound)));
    }

    #[test]
    fn list_by_project_returns_created_processes() {
        let (_dir, service, project_id) = service_with_project();
        service.create(&project_id, "Process A", None).unwrap();
        service.create(&project_id, "Process B", None).unwrap();

        let listed = service.list_by_project(&project_id).expect("list");
        assert_eq!(listed.len(), 2);
    }

    #[test]
    fn get_missing_process_returns_not_found() {
        let (_dir, service, _project_id) = service_with_project();
        assert!(matches!(service.get("does-not-exist"), Err(AppError::NotFound)));
    }

    #[test]
    fn update_succeeds_trims_and_changes_status() {
        let (_dir, service, project_id) = service_with_project();
        let created = service.create(&project_id, "Original", None).unwrap();

        let updated = service
            .update(&created.id, "  Updated name  ", Some("  updated desc  "), "in_progress")
            .expect("update");

        assert_eq!(updated.name, "Updated name");
        assert_eq!(updated.description, "updated desc");
        assert_eq!(updated.status, ProcessStatus::InProgress);
    }

    #[test]
    fn update_rejects_invalid_status() {
        let (_dir, service, project_id) = service_with_project();
        let created = service.create(&project_id, "Original", None).unwrap();
        let result = service.update(&created.id, "Original", None, "not_a_real_status");
        assert!(matches!(result, Err(AppError::Validation(_))));
    }

    #[test]
    fn update_missing_process_returns_not_found() {
        let (_dir, service, _project_id) = service_with_project();
        let result = service.update("does-not-exist", "New name", None, "draft");
        assert!(matches!(result, Err(AppError::NotFound)));
    }

    #[test]
    fn update_regenerates_updated_at_and_preserves_created_at_and_project_id() {
        let (_dir, service, project_id) = service_with_project();
        let created = service.create(&project_id, "Original", None).unwrap();

        std::thread::sleep(std::time::Duration::from_millis(5));

        let updated = service
            .update(&created.id, "Updated", None, "draft")
            .expect("update");

        assert_eq!(updated.id, created.id, "id must not change");
        assert_eq!(updated.project_id, project_id, "project_id must not change");
        assert_eq!(updated.created_at, created.created_at, "created_at must not change");
        assert!(
            updated.updated_at > created.updated_at,
            "updated_at must be regenerated by the backend"
        );
    }

    #[test]
    fn delete_removes_the_process() {
        let (_dir, service, project_id) = service_with_project();
        let created = service.create(&project_id, "Process", None).unwrap();

        service.delete(&created.id).expect("delete");
        assert!(matches!(service.get(&created.id), Err(AppError::NotFound)));
    }

    #[test]
    fn delete_missing_process_returns_not_found() {
        let (_dir, service, _project_id) = service_with_project();
        assert!(matches!(service.delete("does-not-exist"), Err(AppError::NotFound)));
    }
}
