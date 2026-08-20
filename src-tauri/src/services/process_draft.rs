//! `ProcessDraftService`: TASK-017's business logic — the one place
//! that reads a Process's identity, its Captures' metadata, and their
//! screenshot bytes, and assembles them into an `ai::ProcessDraftRequest`
//! before handing off to `ai::AiService`. Sits at the same layer as
//! `CaptureService`/`ProcessService`, but is the first service in this
//! app to depend on a domain-external abstraction (`AiService`) as well
//! as repositories — matches docs/architecture.md §8's "AI feature →
//! AI service abstraction" boundary: this is the "AI feature" side of
//! that line, never reaching past `AiService` into anything
//! OpenAI-specific itself.

use crate::ai::{AiService, CaptureForAi, ProcessDraft, ProcessDraftRequest};
use crate::credentials::CredentialStore;
use crate::errors::AppError;
use crate::media::MediaStorage;
use crate::models::capture::CaptureType;
use crate::models::process_version::ProcessVersion;
use crate::repositories::capture::CaptureRepository;
use crate::repositories::process::ProcessRepository;
use crate::repositories::process_version::ProcessVersionRepository;
use uuid::Uuid;

/// A defensive cap, not a product decision — nothing in roadmap.md
/// TASK-017 asks for a limit on how many Captures one generation call
/// can cover, but sending an unbounded number of screenshot images in
/// one request risks a multi-minute timeout or a runaway API bill for
/// what would likely be a mis-scoped Process anyway. Generous enough
/// that no real single-session Process should ever hit it.
const MAX_CAPTURES: usize = 60;

/// Length limits for a user's edited content (TASK-019) — the same
/// "reject obviously-wrong input, don't constrain legitimate use"
/// rationale, and the same numbers, `CaptureService`'s title/description
/// limits already use, since a step's title/description is the same
/// kind of content.
const MAX_SUMMARY_LENGTH: usize = 5000;
const MAX_STEP_TITLE_LENGTH: usize = 200;
const MAX_STEP_DESCRIPTION_LENGTH: usize = 5000;

pub struct ProcessDraftService {
    ai: Box<dyn AiService>,
    credentials: Box<dyn CredentialStore>,
    processes: Box<dyn ProcessRepository>,
    captures: Box<dyn CaptureRepository>,
    versions: Box<dyn ProcessVersionRepository>,
    media: MediaStorage,
}

impl ProcessDraftService {
    pub fn new(
        ai: Box<dyn AiService>,
        credentials: Box<dyn CredentialStore>,
        processes: Box<dyn ProcessRepository>,
        captures: Box<dyn CaptureRepository>,
        versions: Box<dyn ProcessVersionRepository>,
        media: MediaStorage,
    ) -> Self {
        Self { ai, credentials, processes, captures, versions, media }
    }

    /// Generates a draft and persists it as a new `ProcessVersion` —
    /// never overwriting a previous one (docs/architecture.md §5).
    /// Returns the persisted version (id + timestamp + content), not
    /// the bare `ai::ProcessDraft` TASK-017 originally returned.
    pub fn generate(&self, process_id: &str) -> Result<ProcessVersion, AppError> {
        let api_key = self
            .credentials
            .get_api_key()?
            .ok_or_else(|| AppError::Validation("Save an API key in Settings first.".to_string()))?;

        let process = self.processes.get(process_id)?.ok_or(AppError::NotFound)?;

        let mut captures = self.captures.list_by_process(process_id)?;
        if captures.is_empty() {
            return Err(AppError::Validation(
                "Add at least one capture to this process before generating.".to_string(),
            ));
        }
        if captures.len() > MAX_CAPTURES {
            return Err(AppError::Validation(format!(
                "This process has {} captures — generation is limited to {MAX_CAPTURES} at a time. \
                 Split it into smaller processes.",
                captures.len()
            )));
        }

        // Chronological order (oldest first) — the order the work
        // actually happened in, not `list_by_process`'s own
        // `updated_at DESC` list-display order (see docs/architecture.md
        // §21). This is also the order `capture_indices` in the model's
        // response are numbered against (see `ai::openai::build_request`).
        captures.sort_by_key(|capture| capture.created_at);

        let captures_for_ai = captures
            .into_iter()
            .map(|capture| {
                let screenshot_png = if capture.capture_type == CaptureType::Screenshot {
                    match self.media.read_capture(&capture.id) {
                        Ok(bytes) => Some(bytes),
                        Err(err) => {
                            // Missing media for a screenshot Capture is
                            // unexpected but not fatal to the whole
                            // generation — describe it from title/
                            // description alone, same as a Recording/Note.
                            eprintln!(
                                "[golive] screenshot media missing for capture {}, describing from text only: {err}",
                                capture.id
                            );
                            None
                        }
                    }
                } else {
                    None
                };

                CaptureForAi {
                    id: capture.id,
                    capture_type: capture.capture_type.as_str().to_string(),
                    title: capture.title,
                    description: capture.description,
                    screenshot_png,
                }
            })
            .collect();

        let request = ProcessDraftRequest {
            process_name: process.name,
            process_description: process.description,
            captures: captures_for_ai,
        };

        let content = self.ai.generate_process_draft(&api_key, &request)?;

        let now = now_ms();
        let version =
            ProcessVersion { id: Uuid::new_v4().to_string(), process_id: process_id.to_string(), content, created_at: now, updated_at: now };
        self.versions.create(&version)?;

        Ok(version)
    }

    /// Every version of `process_id`, newest first — TASK-018's
    /// "listable" half of "versions are listable and retrievable". Does
    /// not itself confirm `process_id` exists — an unknown id and a
    /// real process with zero versions both just return an empty list,
    /// same as `CaptureRepository::list_by_process`'s own behavior for
    /// captures.
    pub fn list_versions(&self, process_id: &str) -> Result<Vec<ProcessVersion>, AppError> {
        self.versions.list_by_process(process_id)
    }

    /// One version by its own id — TASK-018's "retrievable" half.
    pub fn get_version(&self, id: &str) -> Result<ProcessVersion, AppError> {
        self.versions.get(id)?.ok_or(AppError::NotFound)
    }

    /// The newest version for `process_id`, or `None` if it's never
    /// been generated — lets a UI show the last real generation on
    /// load instead of starting blank every time a Process is
    /// reopened, which is most of the point of persisting this at all
    /// (see `ProcessVersionRepository::get_latest_by_process`).
    pub fn get_latest_version(&self, process_id: &str) -> Result<Option<ProcessVersion>, AppError> {
        self.versions.get_latest_by_process(process_id)
    }

    /// Saves a user's edits to an existing version's content in place
    /// (TASK-019) — trims/validates, bumps `updated_at`, but never
    /// touches `id`/`process_id`/`created_at`. Deliberately distinct
    /// from `generate`: this never inserts a new row, and `generate`
    /// never calls this — regeneration and editing are two genuinely
    /// different operations on two genuinely different things (a new
    /// AI result vs. a human correction to an existing one), see
    /// DECISIONS.md for the full reasoning.
    pub fn update_version_content(&self, id: &str, content: ProcessDraft) -> Result<ProcessVersion, AppError> {
        let content = validate_draft(content)?;
        let updated_at = now_ms();
        if !self.versions.update_content(id, &content, updated_at)? {
            return Err(AppError::NotFound);
        }
        self.versions.get(id)?.ok_or(AppError::NotFound)
    }
}

fn validate_draft(mut draft: ProcessDraft) -> Result<ProcessDraft, AppError> {
    let summary = draft.summary.trim();
    if summary.is_empty() {
        return Err(AppError::Validation("Summary is required.".to_string()));
    }
    if summary.chars().count() > MAX_SUMMARY_LENGTH {
        return Err(AppError::Validation(format!("Summary must be {MAX_SUMMARY_LENGTH} characters or fewer.")));
    }
    draft.summary = summary.to_string();

    if draft.steps.is_empty() {
        return Err(AppError::Validation("A process draft needs at least one step.".to_string()));
    }
    for step in &mut draft.steps {
        let title = step.title.trim();
        if title.is_empty() {
            return Err(AppError::Validation("Every step needs a title.".to_string()));
        }
        if title.chars().count() > MAX_STEP_TITLE_LENGTH {
            return Err(AppError::Validation(format!(
                "A step's title must be {MAX_STEP_TITLE_LENGTH} characters or fewer."
            )));
        }
        if step.description.chars().count() > MAX_STEP_DESCRIPTION_LENGTH {
            return Err(AppError::Validation(format!(
                "A step's description must be {MAX_STEP_DESCRIPTION_LENGTH} characters or fewer."
            )));
        }
        step.title = title.to_string();
        step.description = step.description.trim().to_string();
    }

    Ok(draft)
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
    use crate::ai::{ProcessDraft, ProcessDraftStep};
    use crate::models::capture::Capture;
    use crate::models::process::{Process, ProcessStatus};
    use std::sync::{Arc, Mutex};

    struct FakeCredentialStore {
        key: Option<&'static str>,
    }
    impl CredentialStore for FakeCredentialStore {
        fn save_api_key(&self, _key: &str) -> Result<(), AppError> {
            unreachable!("not exercised by these tests")
        }
        fn get_api_key(&self) -> Result<Option<String>, AppError> {
            Ok(self.key.map(str::to_string))
        }
        fn clear_api_key(&self) -> Result<(), AppError> {
            unreachable!("not exercised by these tests")
        }
    }

    struct FakeProcessRepository {
        process: Option<Process>,
    }
    impl ProcessRepository for FakeProcessRepository {
        fn create(&self, _process: &Process) -> Result<(), AppError> {
            unreachable!()
        }
        fn list_by_project(&self, _project_id: &str) -> Result<Vec<Process>, AppError> {
            unreachable!()
        }
        fn get(&self, _id: &str) -> Result<Option<Process>, AppError> {
            Ok(self.process.clone())
        }
        fn update(&self, _process: &Process) -> Result<bool, AppError> {
            unreachable!()
        }
        fn delete(&self, _id: &str) -> Result<bool, AppError> {
            unreachable!()
        }
    }

    struct FakeCaptureRepository {
        captures: Vec<Capture>,
    }
    impl CaptureRepository for FakeCaptureRepository {
        fn create(&self, _capture: &Capture) -> Result<(), AppError> {
            unreachable!()
        }
        fn list_by_process(&self, _process_id: &str) -> Result<Vec<Capture>, AppError> {
            Ok(self.captures.clone())
        }
        fn get(&self, _id: &str) -> Result<Option<Capture>, AppError> {
            unreachable!()
        }
        fn update(&self, _capture: &Capture) -> Result<bool, AppError> {
            unreachable!()
        }
        fn delete(&self, _id: &str) -> Result<bool, AppError> {
            unreachable!()
        }
        fn list_all_ids(&self) -> Result<Vec<String>, AppError> {
            unreachable!()
        }
    }

    /// A real in-memory store, not a stub — `generate` persisting
    /// through this (rather than this being a `Mutex<Option<...>>`
    /// single-slot fake) is exactly what
    /// `generate_persists_each_call_as_a_new_version_never_overwriting`
    /// needs to observe.
    #[derive(Clone, Default)]
    struct FakeProcessVersionRepository {
        versions: Arc<Mutex<Vec<ProcessVersion>>>,
    }
    impl ProcessVersionRepository for FakeProcessVersionRepository {
        fn create(&self, version: &ProcessVersion) -> Result<(), AppError> {
            self.versions.lock().unwrap().push(version.clone());
            Ok(())
        }
        fn list_by_process(&self, process_id: &str) -> Result<Vec<ProcessVersion>, AppError> {
            Ok(self
                .versions
                .lock()
                .unwrap()
                .iter()
                .filter(|v| v.process_id == process_id)
                .cloned()
                .collect())
        }
        fn get_latest_by_process(&self, process_id: &str) -> Result<Option<ProcessVersion>, AppError> {
            Ok(self
                .versions
                .lock()
                .unwrap()
                .iter()
                .filter(|v| v.process_id == process_id)
                .max_by_key(|v| v.created_at)
                .cloned())
        }
        fn get(&self, id: &str) -> Result<Option<ProcessVersion>, AppError> {
            Ok(self.versions.lock().unwrap().iter().find(|v| v.id == id).cloned())
        }
        fn update_content(&self, id: &str, content: &ProcessDraft, updated_at: i64) -> Result<bool, AppError> {
            let mut versions = self.versions.lock().unwrap();
            match versions.iter_mut().find(|v| v.id == id) {
                Some(version) => {
                    version.content = content.clone();
                    version.updated_at = updated_at;
                    Ok(true)
                }
                None => Ok(false),
            }
        }
    }

    /// Ids of the captures a `ProcessDraftRequest` carried, in the order
    /// it carried them — everything `RecordingAiService`'s tests need to
    /// assert on (see `generate_sorts_captures_chronologically_before_sending`).
    type RecordedCaptureIds = std::sync::Arc<Mutex<Option<Vec<String>>>>;

    /// Records the capture ordering it was called with (so tests can
    /// assert `ProcessDraftService` sorted before calling out) via a
    /// shared handle obtained from `new_with_handle` — a plain field
    /// wouldn't be readable after the service takes ownership of this as
    /// a `Box<dyn AiService>`. Returns a canned result.
    struct RecordingAiService {
        recorded: RecordedCaptureIds,
    }
    impl RecordingAiService {
        fn new_with_handle() -> (Self, RecordedCaptureIds) {
            let recorded: RecordedCaptureIds = std::sync::Arc::new(Mutex::new(None));
            (Self { recorded: recorded.clone() }, recorded)
        }
        fn new() -> Self {
            Self::new_with_handle().0
        }
    }
    impl AiService for RecordingAiService {
        fn generate_process_draft(&self, _api_key: &str, request: &ProcessDraftRequest) -> Result<ProcessDraft, AppError> {
            *self.recorded.lock().unwrap() = Some(request.captures.iter().map(|c| c.id.clone()).collect());
            Ok(ProcessDraft {
                summary: "generated".to_string(),
                steps: vec![ProcessDraftStep {
                    title: "Step".to_string(),
                    description: "Desc".to_string(),
                    capture_ids: vec![],
                }],
            })
        }
    }

    fn sample_process() -> Process {
        Process {
            id: "proc-1".to_string(),
            project_id: "proj-1".to_string(),
            name: "Kickoff call".to_string(),
            description: "First client call".to_string(),
            status: ProcessStatus::InProgress,
            created_at: 0,
            updated_at: 0,
        }
    }

    fn sample_capture(id: &str, created_at: i64) -> Capture {
        Capture {
            id: id.to_string(),
            process_id: "proc-1".to_string(),
            capture_type: CaptureType::Note,
            title: format!("Capture {id}"),
            description: "".to_string(),
            created_at,
            updated_at: created_at,
        }
    }

    /// Returns the `TempDir` and the `FakeProcessVersionRepository`
    /// alongside the service — the `TempDir` for the same reason
    /// `media::tests::storage_in_temp_dir` returns one (stays alive for
    /// the whole test), the fake repository (cheap to clone — it just
    /// wraps an `Arc`) so a test can inspect what `generate` actually
    /// persisted after the service (which owns the only other handle,
    /// boxed as a trait object) is done with it.
    fn build_service(
        key: Option<&'static str>,
        process: Option<Process>,
        captures: Vec<Capture>,
        ai: RecordingAiService,
    ) -> (ProcessDraftService, tempfile::TempDir, FakeProcessVersionRepository) {
        let dir = tempfile::tempdir().expect("temp dir");
        let versions = FakeProcessVersionRepository::default();
        let service = ProcessDraftService::new(
            Box::new(ai),
            Box::new(FakeCredentialStore { key }),
            Box::new(FakeProcessRepository { process }),
            Box::new(FakeCaptureRepository { captures }),
            Box::new(versions.clone()),
            MediaStorage::init(dir.path()).expect("media storage init"),
        );
        (service, dir, versions)
    }

    #[test]
    fn generate_rejects_when_no_api_key_is_saved() {
        let (service, _dir, _versions) =
            build_service(None, Some(sample_process()), vec![sample_capture("c1", 1)], RecordingAiService::new());
        let result = service.generate("proc-1");
        assert!(matches!(result, Err(AppError::Validation(_))));
    }

    #[test]
    fn generate_rejects_a_missing_process() {
        let (service, _dir, _versions) =
            build_service(Some("sk-test"), None, vec![sample_capture("c1", 1)], RecordingAiService::new());
        let result = service.generate("proc-1");
        assert!(matches!(result, Err(AppError::NotFound)));
    }

    #[test]
    fn generate_rejects_a_process_with_no_captures() {
        let (service, _dir, _versions) =
            build_service(Some("sk-test"), Some(sample_process()), vec![], RecordingAiService::new());
        let result = service.generate("proc-1");
        assert!(matches!(result, Err(AppError::Validation(_))));
    }

    #[test]
    fn generate_rejects_more_than_the_max_capture_count() {
        let captures: Vec<Capture> = (0..(MAX_CAPTURES + 1) as i64).map(|i| sample_capture(&format!("c{i}"), i)).collect();
        let (service, _dir, _versions) =
            build_service(Some("sk-test"), Some(sample_process()), captures, RecordingAiService::new());
        let result = service.generate("proc-1");
        assert!(matches!(result, Err(AppError::Validation(_))));
    }

    #[test]
    fn generate_sorts_captures_chronologically_before_sending() {
        let captures = vec![sample_capture("newest", 300), sample_capture("oldest", 100), sample_capture("middle", 200)];
        let (ai, recorded) = RecordingAiService::new_with_handle();
        let (service, _dir, _versions) = build_service(Some("sk-test"), Some(sample_process()), captures, ai);

        service.generate("proc-1").expect("generate");

        assert_eq!(
            recorded.lock().unwrap().clone(),
            Some(vec!["oldest".to_string(), "middle".to_string(), "newest".to_string()]),
            "captures must be sent oldest-created first, not in the repository's own updated_at-DESC order"
        );
    }

    #[test]
    fn generate_returns_the_persisted_version_wrapping_the_ai_services_draft() {
        let (service, _dir, _versions) = build_service(
            Some("sk-test"),
            Some(sample_process()),
            vec![sample_capture("c1", 1)],
            RecordingAiService::new(),
        );
        let version = service.generate("proc-1").expect("generate");
        assert_eq!(version.process_id, "proc-1");
        assert!(!version.id.is_empty());
        assert_eq!(version.content.steps.len(), 1);
        assert_eq!(version.content.steps[0].title, "Step");
    }

    #[test]
    fn generate_persists_a_version_the_repository_can_retrieve() {
        let (service, _dir, versions) = build_service(
            Some("sk-test"),
            Some(sample_process()),
            vec![sample_capture("c1", 1)],
            RecordingAiService::new(),
        );
        let version = service.generate("proc-1").expect("generate");

        let fetched = versions.get(&version.id).expect("get").expect("version should be persisted");
        assert_eq!(fetched, version);
    }

    #[test]
    fn generate_never_overwrites_a_previous_version_each_call_is_a_new_one() {
        let (service, _dir, versions) = build_service(
            Some("sk-test"),
            Some(sample_process()),
            vec![sample_capture("c1", 1)],
            RecordingAiService::new(),
        );

        let first = service.generate("proc-1").expect("first generate");
        let second = service.generate("proc-1").expect("second generate");

        assert_ne!(first.id, second.id, "each generation must get its own id, never reusing one");
        let listed = versions.list_by_process("proc-1").expect("list");
        assert_eq!(listed.len(), 2, "both versions must remain retrievable — neither call may overwrite the other");
    }

    #[test]
    fn list_versions_delegates_to_the_repository() {
        let (service, _dir, _versions) = build_service(
            Some("sk-test"),
            Some(sample_process()),
            vec![sample_capture("c1", 1)],
            RecordingAiService::new(),
        );
        service.generate("proc-1").expect("generate");
        service.generate("proc-1").expect("generate again");

        let listed = service.list_versions("proc-1").expect("list_versions");
        assert_eq!(listed.len(), 2);
    }

    #[test]
    fn list_versions_for_a_process_with_none_is_empty() {
        let (service, _dir, _versions) = build_service(
            Some("sk-test"),
            Some(sample_process()),
            vec![sample_capture("c1", 1)],
            RecordingAiService::new(),
        );
        assert!(service.list_versions("no-such-process").expect("list_versions").is_empty());
    }

    #[test]
    fn get_version_returns_a_persisted_version() {
        let (service, _dir, _versions) = build_service(
            Some("sk-test"),
            Some(sample_process()),
            vec![sample_capture("c1", 1)],
            RecordingAiService::new(),
        );
        let created = service.generate("proc-1").expect("generate");

        let fetched = service.get_version(&created.id).expect("get_version");
        assert_eq!(fetched, created);
    }

    #[test]
    fn get_version_returns_not_found_for_a_missing_id() {
        let (service, _dir, _versions) = build_service(
            Some("sk-test"),
            Some(sample_process()),
            vec![sample_capture("c1", 1)],
            RecordingAiService::new(),
        );
        let result = service.get_version("does-not-exist");
        assert!(matches!(result, Err(AppError::NotFound)));
    }

    #[test]
    fn get_latest_version_returns_none_before_any_generation() {
        let (service, _dir, _versions) = build_service(
            Some("sk-test"),
            Some(sample_process()),
            vec![sample_capture("c1", 1)],
            RecordingAiService::new(),
        );
        assert!(service.get_latest_version("proc-1").expect("get_latest_version").is_none());
    }

    #[test]
    fn get_latest_version_returns_the_most_recent_generation() {
        let (service, _dir, _versions) = build_service(
            Some("sk-test"),
            Some(sample_process()),
            vec![sample_capture("c1", 1)],
            RecordingAiService::new(),
        );
        service.generate("proc-1").expect("first generate");
        let second = service.generate("proc-1").expect("second generate");

        let latest = service.get_latest_version("proc-1").expect("get_latest_version").expect("should exist");
        assert_eq!(latest.id, second.id);
    }

    fn edited_draft(summary: &str) -> ProcessDraft {
        ProcessDraft {
            summary: summary.to_string(),
            steps: vec![ProcessDraftStep {
                title: "Edited step".to_string(),
                description: "Edited description".to_string(),
                capture_ids: vec![],
            }],
        }
    }

    #[test]
    fn update_version_content_saves_edits_and_returns_the_updated_version() {
        let (service, _dir, _versions) = build_service(
            Some("sk-test"),
            Some(sample_process()),
            vec![sample_capture("c1", 1)],
            RecordingAiService::new(),
        );
        let created = service.generate("proc-1").expect("generate");

        let updated = service.update_version_content(&created.id, edited_draft("Hand-edited summary")).expect("update");

        assert_eq!(updated.id, created.id);
        assert_eq!(updated.content.summary, "Hand-edited summary");
        assert_eq!(updated.content.steps[0].title, "Edited step");
    }

    #[test]
    fn update_version_content_never_creates_a_new_version() {
        let (service, _dir, versions) = build_service(
            Some("sk-test"),
            Some(sample_process()),
            vec![sample_capture("c1", 1)],
            RecordingAiService::new(),
        );
        let created = service.generate("proc-1").expect("generate");

        service.update_version_content(&created.id, edited_draft("Edit one")).expect("update 1");
        service.update_version_content(&created.id, edited_draft("Edit two")).expect("update 2");

        let listed = versions.list_by_process("proc-1").expect("list");
        assert_eq!(listed.len(), 1, "editing must never insert a new row");
        assert_eq!(listed[0].content.summary, "Edit two");
    }

    #[test]
    fn update_version_content_rejects_a_missing_id() {
        let (service, _dir, _versions) = build_service(
            Some("sk-test"),
            Some(sample_process()),
            vec![sample_capture("c1", 1)],
            RecordingAiService::new(),
        );
        let result = service.update_version_content("does-not-exist", edited_draft("x"));
        assert!(matches!(result, Err(AppError::NotFound)));
    }

    #[test]
    fn update_version_content_rejects_an_empty_summary() {
        let (service, _dir, _versions) = build_service(
            Some("sk-test"),
            Some(sample_process()),
            vec![sample_capture("c1", 1)],
            RecordingAiService::new(),
        );
        let created = service.generate("proc-1").expect("generate");
        let result = service.update_version_content(&created.id, edited_draft("   "));
        assert!(matches!(result, Err(AppError::Validation(_))));
    }

    #[test]
    fn update_version_content_rejects_zero_steps() {
        let (service, _dir, _versions) = build_service(
            Some("sk-test"),
            Some(sample_process()),
            vec![sample_capture("c1", 1)],
            RecordingAiService::new(),
        );
        let created = service.generate("proc-1").expect("generate");
        let empty = ProcessDraft { summary: "Summary".to_string(), steps: vec![] };
        let result = service.update_version_content(&created.id, empty);
        assert!(matches!(result, Err(AppError::Validation(_))));
    }

    #[test]
    fn update_version_content_rejects_a_step_with_an_empty_title() {
        let (service, _dir, _versions) = build_service(
            Some("sk-test"),
            Some(sample_process()),
            vec![sample_capture("c1", 1)],
            RecordingAiService::new(),
        );
        let created = service.generate("proc-1").expect("generate");
        let draft = ProcessDraft {
            summary: "Summary".to_string(),
            steps: vec![ProcessDraftStep { title: "  ".to_string(), description: "d".to_string(), capture_ids: vec![] }],
        };
        let result = service.update_version_content(&created.id, draft);
        assert!(matches!(result, Err(AppError::Validation(_))));
    }

    #[test]
    fn update_version_content_trims_summary_and_step_fields() {
        let (service, _dir, _versions) = build_service(
            Some("sk-test"),
            Some(sample_process()),
            vec![sample_capture("c1", 1)],
            RecordingAiService::new(),
        );
        let created = service.generate("proc-1").expect("generate");
        let draft = ProcessDraft {
            summary: "  Padded summary  ".to_string(),
            steps: vec![ProcessDraftStep {
                title: "  Padded title  ".to_string(),
                description: "  Padded description  ".to_string(),
                capture_ids: vec![],
            }],
        };
        let updated = service.update_version_content(&created.id, draft).expect("update");
        assert_eq!(updated.content.summary, "Padded summary");
        assert_eq!(updated.content.steps[0].title, "Padded title");
        assert_eq!(updated.content.steps[0].description, "Padded description");
    }

    #[test]
    fn update_version_content_preserves_capture_ids_untouched() {
        let (service, _dir, _versions) = build_service(
            Some("sk-test"),
            Some(sample_process()),
            vec![sample_capture("c1", 1)],
            RecordingAiService::new(),
        );
        let created = service.generate("proc-1").expect("generate");
        let draft = ProcessDraft {
            summary: "Summary".to_string(),
            steps: vec![ProcessDraftStep {
                title: "Title".to_string(),
                description: "Desc".to_string(),
                capture_ids: vec!["cap-1".to_string(), "cap-2".to_string()],
            }],
        };
        let updated = service.update_version_content(&created.id, draft).expect("update");
        assert_eq!(updated.content.steps[0].capture_ids, vec!["cap-1".to_string(), "cap-2".to_string()]);
    }
}
