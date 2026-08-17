//! `CaptureService`: the Capture domain's business logic — validation,
//! ID/timestamp generation, and process-existence enforcement — sitting
//! between the thin Tauri commands and `CaptureRepository`. See
//! docs/architecture.md ("Business logic boundary").
//!
//! Holds both a `Box<dyn CaptureRepository>` and a
//! `Box<dyn ProcessRepository>`: creating a capture needs to confirm its
//! parent process exists before writing anything, which needs the process
//! repository, not just the capture one — same shape as `ProcessService`
//! holding `ProjectRepository` (see docs/architecture.md, "Project
//! ownership").

use crate::errors::AppError;
use crate::models::capture::{Capture, CaptureType};
use crate::repositories::capture::CaptureRepository;
use crate::repositories::process::ProcessRepository;
use uuid::Uuid;

/// Same limits and Unicode-scalar-value counting rationale as
/// `ProjectService`/`ProcessService` (see there) — kept as this service's
/// own constants rather than a shared import, consistent with each domain
/// service being self-contained.
const MAX_TITLE_LENGTH: usize = 200;
const MAX_DESCRIPTION_LENGTH: usize = 5000;

pub struct CaptureService {
    repository: Box<dyn CaptureRepository>,
    processes: Box<dyn ProcessRepository>,
}

impl CaptureService {
    pub fn new(repository: Box<dyn CaptureRepository>, processes: Box<dyn ProcessRepository>) -> Self {
        Self { repository, processes }
    }

    /// Verifies `process_id` refers to a real process, trims/validates
    /// `title`/`description`, parses `capture_type` (rejecting anything
    /// other than the three defined values), generates the id and
    /// timestamps, and persists. The frontend never supplies `id`,
    /// `created_at`, or `updated_at`.
    pub fn create(
        &self,
        process_id: &str,
        capture_type: &str,
        title: &str,
        description: Option<&str>,
    ) -> Result<Capture, AppError> {
        let title = validate_title(title)?;
        let description = validate_description(description.unwrap_or(""))?;
        let capture_type = CaptureType::parse(capture_type)
            .ok_or_else(|| AppError::Validation("Invalid capture type.".to_string()))?;

        // Confirms the process exists before creating a capture under it.
        // The `process_id` foreign key (ON DELETE CASCADE) would also
        // reject an orphaned insert, but checking here first gives a
        // clear AppError::NotFound instead of a generic database error.
        self.processes.get(process_id)?.ok_or(AppError::NotFound)?;

        let now = now_ms();
        let capture = Capture {
            id: Uuid::new_v4().to_string(),
            process_id: process_id.to_string(),
            capture_type,
            title,
            description,
            created_at: now,
            updated_at: now,
        };

        self.repository.create(&capture)?;
        Ok(capture)
    }

    pub fn list_by_process(&self, process_id: &str) -> Result<Vec<Capture>, AppError> {
        self.repository.list_by_process(process_id)
    }

    pub fn get(&self, id: &str) -> Result<Capture, AppError> {
        self.repository.get(id)?.ok_or(AppError::NotFound)
    }

    /// Trims/validates `title`/`description`, parses `capture_type`,
    /// verifies the capture exists, and regenerates `updated_at`. `id`,
    /// `process_id`, and `created_at` are taken from the existing record —
    /// a capture can never be moved to another process through update.
    pub fn update(
        &self,
        id: &str,
        capture_type: &str,
        title: &str,
        description: Option<&str>,
    ) -> Result<Capture, AppError> {
        let title = validate_title(title)?;
        let description = validate_description(description.unwrap_or(""))?;
        let capture_type = CaptureType::parse(capture_type)
            .ok_or_else(|| AppError::Validation("Invalid capture type.".to_string()))?;

        let existing = self.get(id)?;

        let capture = Capture {
            id: existing.id,
            process_id: existing.process_id,
            capture_type,
            title,
            description,
            created_at: existing.created_at,
            updated_at: now_ms(),
        };

        if self.repository.update(&capture)? {
            Ok(capture)
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

fn validate_title(raw: &str) -> Result<String, AppError> {
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return Err(AppError::Validation("Capture title is required.".to_string()));
    }
    if trimmed.chars().count() > MAX_TITLE_LENGTH {
        return Err(AppError::Validation(format!(
            "Capture title must be {MAX_TITLE_LENGTH} characters or fewer."
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
    use crate::models::process::Process;
    use crate::models::process::ProcessStatus;
    use crate::models::project::Project;
    use crate::repositories::capture::SqliteCaptureRepository;
    use crate::repositories::process::SqliteProcessRepository;
    use crate::repositories::project::ProjectRepository;
    use crate::repositories::project::SqliteProjectRepository;

    fn service_with_process() -> (tempfile::TempDir, CaptureService, String) {
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

        let process = Process {
            id: "process1".to_string(),
            project_id: project.id.clone(),
            name: "Sample process".to_string(),
            description: String::new(),
            status: ProcessStatus::Draft,
            created_at: 1,
            updated_at: 1,
        };
        SqliteProcessRepository::new(db.pool()).create(&process).unwrap();

        let service = CaptureService::new(
            Box::new(SqliteCaptureRepository::new(db.pool())),
            Box::new(SqliteProcessRepository::new(db.pool())),
        );
        (dir, service, process.id)
    }

    #[test]
    fn create_generates_id_and_timestamps() {
        let (_dir, service, process_id) = service_with_process();
        let capture = service
            .create(&process_id, "screenshot", "Order screen", None)
            .expect("create");

        assert!(!capture.id.is_empty());
        assert!(uuid::Uuid::parse_str(&capture.id).is_ok(), "id should be a UUID");
        assert_eq!(capture.process_id, process_id);
        assert_eq!(capture.capture_type, CaptureType::Screenshot);
        assert_eq!(capture.created_at, capture.updated_at);
    }

    #[test]
    fn create_trims_title_and_description() {
        let (_dir, service, process_id) = service_with_process();
        let capture = service
            .create(&process_id, "note", "  Order screen  ", Some("  notes  "))
            .expect("create");

        assert_eq!(capture.title, "Order screen");
        assert_eq!(capture.description, "notes");
    }

    #[test]
    fn create_rejects_empty_title() {
        let (_dir, service, process_id) = service_with_process();
        assert!(matches!(
            service.create(&process_id, "screenshot", "", None),
            Err(AppError::Validation(_))
        ));
    }

    #[test]
    fn create_rejects_whitespace_only_title() {
        let (_dir, service, process_id) = service_with_process();
        assert!(matches!(
            service.create(&process_id, "screenshot", "   ", None),
            Err(AppError::Validation(_))
        ));
    }

    #[test]
    fn create_rejects_title_over_the_length_limit() {
        let (_dir, service, process_id) = service_with_process();
        let too_long = "a".repeat(MAX_TITLE_LENGTH + 1);
        assert!(matches!(
            service.create(&process_id, "screenshot", &too_long, None),
            Err(AppError::Validation(_))
        ));
    }

    #[test]
    fn create_rejects_description_over_the_length_limit() {
        let (_dir, service, process_id) = service_with_process();
        let too_long = "a".repeat(MAX_DESCRIPTION_LENGTH + 1);
        assert!(matches!(
            service.create(&process_id, "screenshot", "Valid title", Some(&too_long)),
            Err(AppError::Validation(_))
        ));
    }

    #[test]
    fn create_accepts_all_valid_capture_types() {
        let (_dir, service, process_id) = service_with_process();
        let screenshot = service.create(&process_id, "screenshot", "A", None).unwrap();
        let recording = service.create(&process_id, "recording", "B", None).unwrap();
        let note = service.create(&process_id, "note", "C", None).unwrap();

        assert_eq!(screenshot.capture_type, CaptureType::Screenshot);
        assert_eq!(recording.capture_type, CaptureType::Recording);
        assert_eq!(note.capture_type, CaptureType::Note);
    }

    #[test]
    fn create_rejects_invalid_capture_type() {
        let (_dir, service, process_id) = service_with_process();
        let result = service.create(&process_id, "video", "Valid title", None);
        assert!(matches!(result, Err(AppError::Validation(_))));
    }

    #[test]
    fn create_rejects_missing_process() {
        let (_dir, service, _process_id) = service_with_process();
        let result = service.create("does-not-exist", "screenshot", "Valid title", None);
        assert!(matches!(result, Err(AppError::NotFound)));
    }

    #[test]
    fn list_by_process_returns_created_captures() {
        let (_dir, service, process_id) = service_with_process();
        service.create(&process_id, "screenshot", "Capture A", None).unwrap();
        service.create(&process_id, "note", "Capture B", None).unwrap();

        let listed = service.list_by_process(&process_id).expect("list");
        assert_eq!(listed.len(), 2);
    }

    #[test]
    fn get_missing_capture_returns_not_found() {
        let (_dir, service, _process_id) = service_with_process();
        assert!(matches!(service.get("does-not-exist"), Err(AppError::NotFound)));
    }

    #[test]
    fn update_succeeds_trims_and_changes_type() {
        let (_dir, service, process_id) = service_with_process();
        let created = service.create(&process_id, "screenshot", "Original", None).unwrap();

        let updated = service
            .update(&created.id, "recording", "  Updated title  ", Some("  updated desc  "))
            .expect("update");

        assert_eq!(updated.title, "Updated title");
        assert_eq!(updated.description, "updated desc");
        assert_eq!(updated.capture_type, CaptureType::Recording);
    }

    #[test]
    fn update_rejects_invalid_capture_type() {
        let (_dir, service, process_id) = service_with_process();
        let created = service.create(&process_id, "screenshot", "Original", None).unwrap();
        let result = service.update(&created.id, "not_a_real_type", "Original", None);
        assert!(matches!(result, Err(AppError::Validation(_))));
    }

    #[test]
    fn update_missing_capture_returns_not_found() {
        let (_dir, service, _process_id) = service_with_process();
        let result = service.update("does-not-exist", "screenshot", "New title", None);
        assert!(matches!(result, Err(AppError::NotFound)));
    }

    #[test]
    fn update_regenerates_updated_at_and_preserves_created_at_id_and_process_id() {
        let (_dir, service, process_id) = service_with_process();
        let created = service.create(&process_id, "screenshot", "Original", None).unwrap();

        std::thread::sleep(std::time::Duration::from_millis(5));

        let updated = service
            .update(&created.id, "screenshot", "Updated", None)
            .expect("update");

        assert_eq!(updated.id, created.id, "id must not change");
        assert_eq!(updated.process_id, process_id, "process_id must not change");
        assert_eq!(updated.created_at, created.created_at, "created_at must not change");
        assert!(
            updated.updated_at > created.updated_at,
            "updated_at must be regenerated by the backend"
        );
    }

    #[test]
    fn delete_removes_the_capture() {
        let (_dir, service, process_id) = service_with_process();
        let created = service.create(&process_id, "screenshot", "Capture", None).unwrap();

        service.delete(&created.id).expect("delete");
        assert!(matches!(service.get(&created.id), Err(AppError::NotFound)));
    }

    #[test]
    fn delete_missing_capture_returns_not_found() {
        let (_dir, service, _process_id) = service_with_process();
        assert!(matches!(service.delete("does-not-exist"), Err(AppError::NotFound)));
    }
}
