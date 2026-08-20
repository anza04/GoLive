//! Thin Tauri-facing commands for AI generation and process versions
//! (TASK-017/TASK-018). Just builds a `ProcessDraftService` from
//! managed state and the real `OpenAiService`/`WindowsCredentialStore`/
//! repository implementations, and delegates — no business logic here,
//! same pattern as every other `commands::*` module.

use crate::ai::openai::OpenAiService;
use crate::ai::ProcessDraft;
use crate::credentials::WindowsCredentialStore;
use crate::db::DbService;
use crate::errors::AppError;
use crate::media::MediaStorage;
use crate::models::process_version::ProcessVersion;
use crate::repositories::capture::SqliteCaptureRepository;
use crate::repositories::process::SqliteProcessRepository;
use crate::repositories::process_version::SqliteProcessVersionRepository;
use crate::services::process_draft::ProcessDraftService;
use serde::Deserialize;
use tauri::State;

/// Same "wrap the single scalar in an explicit input struct" convention
/// `commands::process::ListProcessesInput` documents (see there) —
/// avoids depending on Tauri's default camelCase argument-name
/// conversion for a multi-word parameter name.
#[derive(Deserialize)]
pub struct GenerateProcessDraftInput {
    pub process_id: String,
}

#[derive(Deserialize)]
pub struct ListProcessVersionsInput {
    pub process_id: String,
}

/// TASK-019: the whole edited draft, saved in one call — matches how
/// the frontend editor holds its own local edit buffer (a plain form,
/// not field-by-field autosave) and sends it back as one unit on Save.
#[derive(Deserialize)]
pub struct UpdateProcessVersionContentInput {
    pub id: String,
    pub content: ProcessDraft,
}

fn service(db: &State<DbService>, media: &State<MediaStorage>) -> ProcessDraftService {
    ProcessDraftService::new(
        Box::new(OpenAiService),
        Box::new(WindowsCredentialStore),
        Box::new(SqliteProcessRepository::new(db.pool())),
        Box::new(SqliteCaptureRepository::new(db.pool())),
        Box::new(SqliteProcessVersionRepository::new(db.pool())),
        media.inner().clone(),
    )
}

/// Gathers `process_id`'s Captures (titles, descriptions, and
/// screenshot images where present — see `services::process_draft`),
/// sends them to OpenAI, and persists the result as a new
/// `ProcessVersion` (TASK-018) — never overwriting a previous one, see
/// docs/architecture.md §5. Returns the newly created version.
#[tauri::command]
pub fn generate_process_draft(
    input: GenerateProcessDraftInput,
    db: State<DbService>,
    media: State<MediaStorage>,
) -> Result<ProcessVersion, AppError> {
    service(&db, &media).generate(&input.process_id)
}

/// Every version of `process_id`, newest first.
#[tauri::command]
pub fn list_process_versions(
    input: ListProcessVersionsInput,
    db: State<DbService>,
    media: State<MediaStorage>,
) -> Result<Vec<ProcessVersion>, AppError> {
    service(&db, &media).list_versions(&input.process_id)
}

/// One version by its own id.
#[tauri::command]
pub fn get_process_version(id: String, db: State<DbService>, media: State<MediaStorage>) -> Result<ProcessVersion, AppError> {
    service(&db, &media).get_version(&id)
}

/// The newest version for `process_id`, or `null` if it's never been
/// generated — lets the UI show the last real generation on load
/// instead of starting blank every time a Process is reopened.
#[tauri::command]
pub fn get_latest_process_version(
    input: ListProcessVersionsInput,
    db: State<DbService>,
    media: State<MediaStorage>,
) -> Result<Option<ProcessVersion>, AppError> {
    service(&db, &media).get_latest_version(&input.process_id)
}

/// Saves a user's edits to an existing version's content in place
/// (TASK-019) — never inserts a new version; regeneration
/// (`generate_process_draft`) is the only command that does that. See
/// `services::process_draft::ProcessDraftService::update_version_content`.
#[tauri::command]
pub fn update_process_version_content(
    input: UpdateProcessVersionContentInput,
    db: State<DbService>,
    media: State<MediaStorage>,
) -> Result<ProcessVersion, AppError> {
    service(&db, &media).update_version_content(&input.id, input.content)
}
