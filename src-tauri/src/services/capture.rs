//! `CaptureService`: the Capture domain's business logic — validation,
//! ID/timestamp generation, process-existence enforcement, and (TASK-009)
//! screenshot media orchestration — sitting between the thin Tauri
//! commands and `CaptureRepository`. See docs/architecture.md ("Business
//! logic boundary").
//!
//! Holds a `Box<dyn CaptureRepository>` and a `Box<dyn ProcessRepository>`
//! (creating a capture needs to confirm its parent process exists before
//! writing anything — same shape as `ProcessService` holding
//! `ProjectRepository`), plus (TASK-009) a `MediaStorage` and a
//! `Box<dyn ScreenshotEngine>` for the one capture type that has real
//! media. Metadata-only operations (`create`/`list_by_process`/`get`/
//! `update`) never touch either of the media dependencies except
//! `delete`, which also cleans up a capture's media file if it has one.

use crate::errors::AppError;
use crate::media::MediaStorage;
use crate::models::capture::{Capture, CaptureType};
use crate::native::screenshot::ScreenshotEngine;
use crate::repositories::capture::CaptureRepository;
use crate::repositories::process::ProcessRepository;
use std::collections::HashSet;
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
    media: MediaStorage,
    screenshot_engine: Box<dyn ScreenshotEngine>,
}

impl CaptureService {
    pub fn new(
        repository: Box<dyn CaptureRepository>,
        processes: Box<dyn ProcessRepository>,
        media: MediaStorage,
        screenshot_engine: Box<dyn ScreenshotEngine>,
    ) -> Self {
        Self {
            repository,
            processes,
            media,
            screenshot_engine,
        }
    }

    /// Verifies `process_id` refers to a real process, trims/validates
    /// `title`/`description`, parses `capture_type` (rejecting anything
    /// other than the three defined values), generates the id and
    /// timestamps, and persists. The frontend never supplies `id`,
    /// `created_at`, or `updated_at`. No media is involved — this is the
    /// generic metadata-only path used for all three capture types (see
    /// `create_screenshot` for the one that also captures real media).
    pub fn create(
        &self,
        process_id: &str,
        capture_type: &str,
        title: &str,
        description: Option<&str>,
    ) -> Result<Capture, AppError> {
        let (title, description) = self.validate_for_create(process_id, title, description)?;
        let capture_type = CaptureType::parse(capture_type)
            .ok_or_else(|| AppError::Validation("Invalid capture type.".to_string()))?;

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

    /// Captures the primary display, stores the PNG, and creates the
    /// Capture metadata row — always `CaptureType::Screenshot`; there is
    /// no way for a caller to request a screenshot operation that ends up
    /// typed as `recording`/`note` (see docs/architecture.md, "Screenshot
    /// creation is transactional").
    ///
    /// Application-level "transaction": the screen is captured first (if
    /// this fails, nothing has been created yet — no cleanup needed), the
    /// PNG is written to disk next, and only then is the metadata row
    /// inserted. If the metadata insert fails after the PNG was already
    /// written, the just-written file is deleted so no orphan media is
    /// left behind. There is no real database transaction here — the
    /// media is filesystem data SQLite doesn't participate in — this is
    /// deliberate service-level orchestration instead.
    pub fn create_screenshot(
        &self,
        process_id: &str,
        title: &str,
        description: Option<&str>,
    ) -> Result<Capture, AppError> {
        let (title, description) = self.validate_for_create(process_id, title, description)?;

        // Nothing has been created yet — if capture fails, there's
        // nothing to roll back.
        let png_bytes = self.screenshot_engine.capture_primary_display()?;

        let id = Uuid::new_v4().to_string();

        // Written before the metadata row so a metadata-creation failure
        // has something concrete to clean up (below); the alternative
        // order (row first) would instead risk a Capture whose media was
        // never written at all, which is worse — a metadata row implying
        // media exists when it doesn't, rather than a harmless orphan
        // file nothing references yet.
        self.media.save_capture(&id, &png_bytes)?;

        let now = now_ms();
        let capture = Capture {
            id: id.clone(),
            process_id: process_id.to_string(),
            capture_type: CaptureType::Screenshot,
            title,
            description,
            created_at: now,
            updated_at: now,
        };

        if let Err(err) = self.repository.create(&capture) {
            if let Err(cleanup_err) = self.media.delete_capture(&id) {
                eprintln!("[golive] failed to clean up orphaned screenshot media for {id}: {cleanup_err}");
            }
            return Err(err);
        }

        Ok(capture)
    }

    /// Trims/validates `title`/`description` and confirms `process_id`
    /// refers to a real process — the checks every Capture-creation path
    /// shares (`create`, `create_screenshot`, and TASK-013's two-phase
    /// recording flow below). Extracted so all three apply the exact
    /// same rules rather than three independent copies drifting apart.
    fn validate_for_create(
        &self,
        process_id: &str,
        title: &str,
        description: Option<&str>,
    ) -> Result<(String, String), AppError> {
        let title = validate_title(title)?;
        let description = validate_description(description.unwrap_or(""))?;

        // Confirms the process exists before creating a capture under it.
        // The `process_id` foreign key (ON DELETE CASCADE) would also
        // reject an orphaned insert, but checking here first gives a
        // clear AppError::NotFound instead of a generic database error.
        self.processes.get(process_id)?.ok_or(AppError::NotFound)?;

        Ok((title, description))
    }

    /// Validates a `start_recording_capture` request — same rules as
    /// `create`/`create_screenshot` (title/description trimming and
    /// length limits, process must exist) — *before* a real recording is
    /// started, so an invalid request never starts one that would then
    /// need to be aborted and cleaned up. Returns the trimmed
    /// `(title, description)`; the caller (`commands::recording`) carries
    /// them forward to `finalize_recording` once the recording stops.
    pub fn validate_recording_start(
        &self,
        process_id: &str,
        title: &str,
        description: Option<&str>,
    ) -> Result<(String, String), AppError> {
        self.validate_for_create(process_id, title, description)
    }

    /// Creates the Capture metadata row for a just-finished recording —
    /// the second half of TASK-013's two-phase flow
    /// (`native::recording::RecordingEngine` already wrote the video
    /// file to `media.video_path(id)` while the recording was in
    /// progress; `commands::recording::stop_recording_capture` calls
    /// this only after successfully stopping/finalizing it). Unlike
    /// `create`/`create_screenshot`, `id` is supplied by the caller
    /// rather than generated here — the metadata row must use the same
    /// id the video file was already written under, not a fresh one.
    ///
    /// Confirms the video file actually exists first (a defensive check:
    /// a recording stopped the instant after it started could plausibly
    /// produce no frames/no file — see docs/architecture.md). If the
    /// metadata insert then fails, the video file is cleaned up, the
    /// same discipline `create_screenshot` established for PNGs.
    pub fn finalize_recording(
        &self,
        id: &str,
        process_id: &str,
        title: &str,
        description: &str,
    ) -> Result<Capture, AppError> {
        if !self.media.video_exists(id) {
            return Err(AppError::Capture(
                "The recording finished but produced no video file.".to_string(),
            ));
        }

        let now = now_ms();
        let capture = Capture {
            id: id.to_string(),
            process_id: process_id.to_string(),
            capture_type: CaptureType::Recording,
            title: title.to_string(),
            description: description.to_string(),
            created_at: now,
            updated_at: now,
        };

        if let Err(err) = self.repository.create(&capture) {
            if let Err(cleanup_err) = self.media.delete_video(id) {
                eprintln!("[golive] failed to clean up orphaned recording media for {id}: {cleanup_err}");
            }
            return Err(err);
        }

        Ok(capture)
    }

    pub fn list_by_process(&self, process_id: &str) -> Result<Vec<Capture>, AppError> {
        self.repository.list_by_process(process_id)
    }

    pub fn get(&self, id: &str) -> Result<Capture, AppError> {
        self.repository.get(id)?.ok_or(AppError::NotFound)
    }

    /// Returns the PNG bytes for a screenshot Capture. Returns
    /// `AppError::NotFound` both when `id` isn't a Capture at all and
    /// when it is one but has no media (e.g. a Note/Recording capture,
    /// or a screenshot Capture edited to another type — see
    /// docs/architecture.md, "Editing a screenshot Capture") — the
    /// frontend treats both the same way (no preview to show).
    pub fn get_screenshot_media(&self, id: &str) -> Result<Vec<u8>, AppError> {
        self.media.read_capture(id)
    }

    /// Returns the MP4 bytes for a Recording Capture (TASK-014
    /// playback) — the Recording counterpart to `get_screenshot_media`,
    /// same `AppError::NotFound`-for-no-media behavior (nonexistent
    /// Capture, or one with no video — e.g. a Note/Screenshot, or a
    /// Recording Capture since edited to another type).
    pub fn get_recording_media(&self, id: &str) -> Result<Vec<u8>, AppError> {
        self.media.read_video(id)
    }

    /// Trims/validates `title`/`description`, parses `capture_type`,
    /// verifies the capture exists, and regenerates `updated_at`. `id`,
    /// `process_id`, and `created_at` are taken from the existing record —
    /// a capture can never be moved to another process through update.
    ///
    /// Editing `capture_type` away from `screenshot` (or to it) never
    /// touches media: a screenshot's PNG is keyed by `Capture.id` alone,
    /// so it simply stops (or starts) being shown by
    /// `get_screenshot_media`/the frontend preview without being deleted,
    /// moved, or recreated — see docs/architecture.md and DECISIONS.md
    /// for why this was the deliberate choice over silently deleting or
    /// replacing media on a type change.
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

    /// Deletes the Capture's metadata row, then best-effort deletes its
    /// media file — both the PNG (screenshot) and MP4 (recording, TASK-013)
    /// paths are attempted unconditionally; a Capture is never both, and
    /// `MediaStorage::delete_capture`/`delete_video` are graceful no-ops
    /// for a missing file, so this needs no branch on `capture_type`. If
    /// the metadata delete itself reports "not found", media is left
    /// untouched (see docs/architecture.md, "Delete behavior") — there's
    /// deliberately no "delete the file first" path. A genuine
    /// media-cleanup failure (as opposed to "already missing") is
    /// logged, not surfaced: the metadata — the source of truth for
    /// whether a Capture exists — is already gone, which is what a
    /// successful delete means to the user; raw filesystem errors are
    /// never shown to the frontend.
    pub fn delete(&self, id: &str) -> Result<(), AppError> {
        if self.repository.delete(id)? {
            if let Err(err) = self.media.delete_capture(id) {
                eprintln!("[golive] failed to delete capture media for {id}: {err}");
            }
            if let Err(err) = self.media.delete_video(id) {
                eprintln!("[golive] failed to delete recording media for {id}: {err}");
            }
            Ok(())
        } else {
            Err(AppError::NotFound)
        }
    }

    /// Sweeps the media directory for PNG files whose Capture no longer
    /// exists in the database — the cleanup boundary the Project/Process
    /// cascade limitation relies on (see docs/architecture.md, "Cascade
    /// media cleanup"). Intended to be called once at application
    /// startup (see `lib.rs`), never per-screenshot. Returns the number
    /// of orphaned files removed.
    pub fn reconcile_media(&self) -> Result<usize, AppError> {
        let known_ids: HashSet<String> = self.repository.list_all_ids()?.into_iter().collect();
        self.media.reconcile(&known_ids)
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
    use std::sync::atomic::{AtomicUsize, Ordering};

    /// A deterministic `ScreenshotEngine` test double — real screen
    /// capture can't be exercised headlessly/deterministically in
    /// automated tests (see docs/architecture.md, "Screenshot capture
    /// testing limitation"), so these tests prove the *orchestration*
    /// (transactional create, cleanup-on-failure, delete, reconciliation)
    /// against a fake engine instead of the real `WindowsScreenshotEngine`.
    struct FakeScreenshotEngine {
        png_bytes: Vec<u8>,
        calls: AtomicUsize,
    }

    impl FakeScreenshotEngine {
        fn new(png_bytes: Vec<u8>) -> Self {
            Self { png_bytes, calls: AtomicUsize::new(0) }
        }
    }

    impl ScreenshotEngine for FakeScreenshotEngine {
        fn capture_primary_display(&self) -> Result<Vec<u8>, AppError> {
            self.calls.fetch_add(1, Ordering::SeqCst);
            Ok(self.png_bytes.clone())
        }
    }

    struct FailingScreenshotEngine;

    impl ScreenshotEngine for FailingScreenshotEngine {
        fn capture_primary_display(&self) -> Result<Vec<u8>, AppError> {
            Err(AppError::Capture("No display is available to capture.".to_string()))
        }
    }

    /// A `CaptureRepository` decorator whose `create` always fails after
    /// delegating everything else to a real `SqliteCaptureRepository` —
    /// used to prove `create_screenshot` cleans up the PNG it already
    /// wrote when the metadata insert fails.
    struct FailingCreateCaptureRepository {
        inner: SqliteCaptureRepository,
    }

    impl CaptureRepository for FailingCreateCaptureRepository {
        fn create(&self, _capture: &Capture) -> Result<(), AppError> {
            Err(AppError::Database)
        }

        fn list_by_process(&self, process_id: &str) -> Result<Vec<Capture>, AppError> {
            self.inner.list_by_process(process_id)
        }

        fn get(&self, id: &str) -> Result<Option<Capture>, AppError> {
            self.inner.get(id)
        }

        fn update(&self, capture: &Capture) -> Result<bool, AppError> {
            self.inner.update(capture)
        }

        fn delete(&self, id: &str) -> Result<bool, AppError> {
            self.inner.delete(id)
        }

        fn list_all_ids(&self) -> Result<Vec<String>, AppError> {
            self.inner.list_all_ids()
        }
    }

    fn service_with_process() -> (tempfile::TempDir, CaptureService, String) {
        let (dir, service, process_id, _media_dir) = service_with_process_and_media();
        (dir, service, process_id)
    }

    fn service_with_process_and_media() -> (tempfile::TempDir, CaptureService, String, std::path::PathBuf) {
        let dir = tempfile::tempdir().expect("temp dir");
        let db = DbService::init(dir.path()).expect("db init");
        let media = crate::media::MediaStorage::init(dir.path()).expect("media init");
        let media_dir = dir.path().join("captures");

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
            media,
            Box::new(FakeScreenshotEngine::new(vec![0x89, 0x50, 0x4e, 0x47, 1, 2, 3])),
        );
        (dir, service, process.id, media_dir)
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
    fn create_metadata_only_never_touches_media() {
        let (_dir, service, process_id, media_dir) = service_with_process_and_media();
        service.create(&process_id, "screenshot", "Not a real screenshot", None).unwrap();
        // The generic metadata `create` path must never call the
        // screenshot engine or write a file — only `create_screenshot`
        // does.
        assert!(!media_dir.exists() || std::fs::read_dir(&media_dir).unwrap().next().is_none());
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
    fn update_changing_type_away_from_screenshot_does_not_delete_media() {
        let (_dir, service, process_id) = service_with_process();
        let created = service.create_screenshot(&process_id, "Original", None).unwrap();
        assert!(service.get_screenshot_media(&created.id).is_ok(), "media should exist right after capture");

        let updated = service.update(&created.id, "note", "Original", None).unwrap();
        assert_eq!(updated.capture_type, CaptureType::Note);

        // The PNG is keyed by id, not type — changing the type away from
        // screenshot must not silently delete or move it (see
        // docs/architecture.md, "Editing a screenshot Capture").
        assert!(
            service.get_screenshot_media(&created.id).is_ok(),
            "media must survive a type change away from screenshot"
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

    #[test]
    fn create_screenshot_captures_saves_media_and_sets_type() {
        let (_dir, service, process_id) = service_with_process();
        let capture = service.create_screenshot(&process_id, "My screenshot", Some("desc")).unwrap();

        assert_eq!(capture.capture_type, CaptureType::Screenshot);
        assert_eq!(capture.title, "My screenshot");

        let media = service.get_screenshot_media(&capture.id).expect("media should exist");
        assert_eq!(media, vec![0x89, 0x50, 0x4e, 0x47, 1, 2, 3]);
    }

    #[test]
    fn create_screenshot_rejects_missing_process() {
        let (_dir, service, _process_id) = service_with_process();
        let result = service.create_screenshot("does-not-exist", "Title", None);
        assert!(matches!(result, Err(AppError::NotFound)));
    }

    #[test]
    fn create_screenshot_rejects_empty_title() {
        let (_dir, service, process_id) = service_with_process();
        assert!(matches!(
            service.create_screenshot(&process_id, "", None),
            Err(AppError::Validation(_))
        ));
    }

    #[test]
    fn create_screenshot_leaves_no_orphan_capture_when_engine_fails() {
        let dir = tempfile::tempdir().expect("temp dir");
        let db = DbService::init(dir.path()).expect("db init");
        let media = crate::media::MediaStorage::init(dir.path()).expect("media init");

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
            media,
            Box::new(FailingScreenshotEngine),
        );

        let result = service.create_screenshot(&process.id, "Title", None);
        assert!(matches!(result, Err(AppError::Capture(_))));
        assert_eq!(service.list_by_process(&process.id).unwrap().len(), 0, "no orphan Capture row");
        assert!(
            dir.path().join("captures").read_dir().map(|mut d| d.next().is_none()).unwrap_or(true),
            "no orphan PNG file"
        );
    }

    #[test]
    fn create_screenshot_cleans_up_the_png_when_metadata_insert_fails() {
        let dir = tempfile::tempdir().expect("temp dir");
        let db = DbService::init(dir.path()).expect("db init");
        let media = crate::media::MediaStorage::init(dir.path()).expect("media init");
        let media_dir = dir.path().join("captures");

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
            Box::new(FailingCreateCaptureRepository { inner: SqliteCaptureRepository::new(db.pool()) }),
            Box::new(SqliteProcessRepository::new(db.pool())),
            media,
            Box::new(FakeScreenshotEngine::new(vec![9, 9, 9])),
        );

        let result = service.create_screenshot(&process.id, "Title", None);
        assert!(matches!(result, Err(AppError::Database)));

        // The PNG was written (the fake repository only fails on
        // `create`), but must have been cleaned up afterward — no file
        // left in the media directory.
        let remaining: Vec<_> = std::fs::read_dir(&media_dir).unwrap().collect();
        assert!(remaining.is_empty(), "orphaned PNG must be cleaned up after a metadata-insert failure");
    }

    #[test]
    fn delete_removes_screenshot_media_alongside_metadata() {
        let (_dir, service, process_id) = service_with_process();
        let capture = service.create_screenshot(&process_id, "Title", None).unwrap();
        assert!(service.get_screenshot_media(&capture.id).is_ok());

        service.delete(&capture.id).expect("delete");

        assert!(matches!(service.get(&capture.id), Err(AppError::NotFound)));
        assert!(matches!(service.get_screenshot_media(&capture.id), Err(AppError::NotFound)));
    }

    #[test]
    fn delete_of_a_note_capture_with_no_media_succeeds_gracefully() {
        let (_dir, service, process_id) = service_with_process();
        let capture = service.create(&process_id, "note", "Just a note", None).unwrap();

        // Must not error even though no media file was ever written for
        // this capture.
        service.delete(&capture.id).expect("delete of metadata-only capture should succeed");
    }

    #[test]
    fn validate_recording_start_trims_and_rejects_like_create() {
        let (_dir, service, process_id) = service_with_process();

        let (title, description) = service
            .validate_recording_start(&process_id, "  My recording  ", Some("  notes  "))
            .expect("validate");
        assert_eq!(title, "My recording");
        assert_eq!(description, "notes");

        assert!(matches!(
            service.validate_recording_start(&process_id, "", None),
            Err(AppError::Validation(_))
        ));
        assert!(matches!(
            service.validate_recording_start("does-not-exist", "Title", None),
            Err(AppError::NotFound)
        ));
    }

    #[test]
    fn finalize_recording_creates_a_recording_capture_using_the_given_id() {
        let (_dir, service, process_id, media_dir) = service_with_process_and_media();
        let id = uuid::Uuid::new_v4().to_string();

        // Stand in for what `native::recording::RecordingEngine` would
        // have already written to disk by the time `stop_recording_capture`
        // calls `finalize_recording`.
        std::fs::write(media_dir.join(format!("{id}.mp4")), b"fake mp4 bytes").unwrap();

        let capture = service.finalize_recording(&id, &process_id, "My recording", "notes").unwrap();

        assert_eq!(capture.id, id);
        assert_eq!(capture.process_id, process_id);
        assert_eq!(capture.capture_type, CaptureType::Recording);
        assert_eq!(capture.title, "My recording");
        assert_eq!(capture.description, "notes");
        assert_eq!(capture.created_at, capture.updated_at);

        let listed = service.list_by_process(&process_id).unwrap();
        assert_eq!(listed.len(), 1);
    }

    #[test]
    fn finalize_recording_fails_when_no_video_file_was_written() {
        let (_dir, service, process_id) = service_with_process();
        let id = uuid::Uuid::new_v4().to_string();

        let result = service.finalize_recording(&id, &process_id, "Title", "");
        assert!(matches!(result, Err(AppError::Capture(_))));
        assert_eq!(service.list_by_process(&process_id).unwrap().len(), 0, "no orphan Capture row");
    }

    #[test]
    fn finalize_recording_cleans_up_the_video_when_metadata_insert_fails() {
        let dir = tempfile::tempdir().expect("temp dir");
        let db = DbService::init(dir.path()).expect("db init");
        let media = crate::media::MediaStorage::init(dir.path()).expect("media init");

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

        let id = uuid::Uuid::new_v4().to_string();
        std::fs::write(media.video_path(&id).unwrap(), b"fake mp4 bytes").unwrap();

        let service = CaptureService::new(
            Box::new(FailingCreateCaptureRepository { inner: SqliteCaptureRepository::new(db.pool()) }),
            Box::new(SqliteProcessRepository::new(db.pool())),
            media.clone(),
            Box::new(FakeScreenshotEngine::new(vec![9, 9, 9])),
        );

        let result = service.finalize_recording(&id, &process.id, "Title", "");
        assert!(matches!(result, Err(AppError::Database)));
        assert!(!media.video_exists(&id), "orphaned video must be cleaned up after a metadata-insert failure");
    }

    #[test]
    fn delete_removes_recording_media_alongside_metadata() {
        let (_dir, service, process_id, media_dir) = service_with_process_and_media();
        let id = uuid::Uuid::new_v4().to_string();
        let video_path = media_dir.join(format!("{id}.mp4"));
        std::fs::write(&video_path, b"fake mp4 bytes").unwrap();
        let capture = service.finalize_recording(&id, &process_id, "Title", "").unwrap();
        assert!(video_path.exists());

        service.delete(&capture.id).expect("delete");

        assert!(matches!(service.get(&capture.id), Err(AppError::NotFound)));
        assert!(!video_path.exists(), "video file must be removed on delete");
    }

    #[test]
    fn get_recording_media_returns_the_video_bytes() {
        let (_dir, service, process_id, media_dir) = service_with_process_and_media();
        let id = uuid::Uuid::new_v4().to_string();
        std::fs::write(media_dir.join(format!("{id}.mp4")), b"fake mp4 bytes").unwrap();
        let capture = service.finalize_recording(&id, &process_id, "Title", "").unwrap();

        let media = service.get_recording_media(&capture.id).expect("media should exist");
        assert_eq!(media, b"fake mp4 bytes");
    }

    #[test]
    fn get_recording_media_for_a_capture_with_no_video_returns_not_found() {
        let (_dir, service, _process_id) = service_with_process();
        // A well-formed UUID, but no video file was ever written for it —
        // mirrors `media::tests::read_missing_video_returns_not_found`.
        let result = service.get_recording_media(&uuid::Uuid::new_v4().to_string());
        assert!(matches!(result, Err(AppError::NotFound)));
    }

    #[test]
    fn reconcile_media_removes_files_orphaned_by_a_cascade_delete() {
        // Builds the service manually (rather than via the
        // `service_with_process*` helpers) so this test can also reach
        // the underlying `SqliteCaptureRepository` directly and delete a
        // row the way SQLite's `ON DELETE CASCADE` would — without going
        // through `CaptureService::delete`, which already cleans up
        // media itself and so wouldn't leave anything to reconcile.
        let dir = tempfile::tempdir().expect("temp dir");
        let db = DbService::init(dir.path()).expect("db init");
        let media = crate::media::MediaStorage::init(dir.path()).expect("media init");

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
            media,
            Box::new(FakeScreenshotEngine::new(vec![1, 2, 3])),
        );

        let kept = service.create_screenshot(&process.id, "Kept", None).unwrap();
        let orphaned = service.create_screenshot(&process.id, "Will be orphaned", None).unwrap();

        // Removes only the metadata row, exactly as a Process/Process's
        // Project cascade delete would (see docs/architecture.md,
        // "Cascade media cleanup") — `orphaned`'s PNG is deliberately
        // left behind on disk to simulate that documented limitation.
        let raw_repo = SqliteCaptureRepository::new(db.pool());
        assert!(CaptureRepository::delete(&raw_repo, &orphaned.id).unwrap());

        let removed = service.reconcile_media().expect("reconcile");

        assert_eq!(removed, 1);
        assert!(service.get_screenshot_media(&kept.id).is_ok(), "kept capture's media must survive");
        assert!(matches!(
            service.get_screenshot_media(&orphaned.id),
            Err(AppError::NotFound)
        ));
    }
}
