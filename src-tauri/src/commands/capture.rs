//! Thin Tauri-facing commands for the Capture domain. Each one just
//! builds a `CaptureService` from the managed `DbService`/`MediaStorage`
//! and delegates — no business logic, no SQL, no filesystem/screenshot
//! code (same pattern as `commands::process`).

use crate::db::DbService;
use crate::errors::AppError;
use crate::media::MediaStorage;
use crate::models::capture::Capture;
use crate::native::screenshot::WindowsScreenshotEngine;
use crate::repositories::capture::SqliteCaptureRepository;
use crate::repositories::process::SqliteProcessRepository;
use crate::services::capture::CaptureService;
use serde::Deserialize;
use tauri::{AppHandle, Emitter, State};

/// Emitted (via `AppHandle::emit`, Rust-side — not gated by the frontend
/// ACL) to every window whenever a Capture is created, however it was
/// created — the generic metadata path, the real-media screenshot path,
/// or (see `commands::recording::stop_recording_capture`) a finished
/// recording. A bugfix, not an original design: without this, a Capture
/// created from the floating widget (a hotkey screenshot, a quick
/// marker, a recording stopped from there) never appeared in the main
/// window's already-open Captures section until it was remounted (see
/// DECISIONS.md). Payload is the full `Capture` — cheap to send, and
/// lets a listener splice it directly into its list without a second
/// round trip.
pub const CAPTURE_CREATED_EVENT: &str = "capture-created";

#[derive(Deserialize)]
pub struct CreateCaptureInput {
    pub process_id: String,
    pub capture_type: String,
    pub title: String,
    #[serde(default)]
    pub description: Option<String>,
}

/// Wraps the single `process_id` scalar in an explicit input struct —
/// same rationale as `commands::process::ListProcessesInput` (see there).
#[derive(Deserialize)]
pub struct ListCapturesInput {
    pub process_id: String,
}

/// Explicit update input — deliberately not the `Capture` model itself,
/// so an update request has no field for `process_id`/`created_at`/
/// `updated_at` to occupy. A capture's process cannot be changed through
/// update.
#[derive(Deserialize)]
pub struct UpdateCaptureInput {
    pub id: String,
    pub capture_type: String,
    pub title: String,
    #[serde(default)]
    pub description: Option<String>,
}

/// Input for the one operation that creates real media (TASK-009). No
/// `capture_type` field exists here at all — a screenshot operation
/// always produces `CaptureType::Screenshot`; there is structurally no
/// way for the frontend to request a screenshot capture typed as
/// `recording`/`note`. No filesystem path of any kind is accepted either
/// — the backend determines storage location, filename, id, and
/// timestamps entirely on its own (see docs/architecture.md, "Safe media
/// access").
#[derive(Deserialize)]
pub struct CreateScreenshotInput {
    pub process_id: String,
    pub title: String,
    #[serde(default)]
    pub description: Option<String>,
}

fn service(db: &State<DbService>, media: &State<MediaStorage>) -> CaptureService {
    CaptureService::new(
        Box::new(SqliteCaptureRepository::new(db.pool())),
        Box::new(SqliteProcessRepository::new(db.pool())),
        media.inner().clone(),
        Box::new(WindowsScreenshotEngine),
    )
}

#[tauri::command]
pub fn create_capture(
    input: CreateCaptureInput,
    db: State<DbService>,
    media: State<MediaStorage>,
    app: AppHandle,
) -> Result<Capture, AppError> {
    let capture =
        service(&db, &media).create(&input.process_id, &input.capture_type, &input.title, input.description.as_deref())?;
    if let Err(err) = app.emit(CAPTURE_CREATED_EVENT, &capture) {
        eprintln!("[golive] failed to broadcast capture creation: {err}");
    }
    Ok(capture)
}

/// Captures the primary display, stores the PNG, and creates the Capture
/// record — the only command that produces real media (see
/// `services::capture::CaptureService::create_screenshot`).
#[tauri::command]
pub fn create_screenshot_capture(
    input: CreateScreenshotInput,
    db: State<DbService>,
    media: State<MediaStorage>,
    app: AppHandle,
) -> Result<Capture, AppError> {
    let capture = service(&db, &media).create_screenshot(&input.process_id, &input.title, input.description.as_deref())?;
    if let Err(err) = app.emit(CAPTURE_CREATED_EVENT, &capture) {
        eprintln!("[golive] failed to broadcast capture creation: {err}");
    }
    Ok(capture)
}

#[tauri::command]
pub fn list_captures(
    input: ListCapturesInput,
    db: State<DbService>,
    media: State<MediaStorage>,
) -> Result<Vec<Capture>, AppError> {
    service(&db, &media).list_by_process(&input.process_id)
}

#[tauri::command]
pub fn get_capture(id: String, db: State<DbService>, media: State<MediaStorage>) -> Result<Capture, AppError> {
    service(&db, &media).get(&id)
}

/// Returns a screenshot Capture's PNG bytes as a raw binary IPC response
/// (`tauri::ipc::Response`) rather than a JSON byte array — the
/// recommended Tauri 2 mechanism for transferring binary data
/// efficiently, and the "dedicated backend media-read command" approach
/// docs/architecture.md ("Safe media access") calls for: the frontend
/// supplies only a Capture id, never a filesystem path, and the backend
/// derives the actual file from that id. Returns `AppError::NotFound` for
/// both a nonexistent Capture and one with no media (Note/Recording, or a
/// screenshot Capture since edited to another type) — the frontend treats
/// both identically (no preview to show).
#[tauri::command]
pub fn get_capture_media(
    id: String,
    db: State<DbService>,
    media: State<MediaStorage>,
) -> Result<tauri::ipc::Response, AppError> {
    let bytes = service(&db, &media).get_screenshot_media(&id)?;
    Ok(tauri::ipc::Response::new(bytes))
}

/// The Recording counterpart to `get_capture_media` (TASK-014): returns
/// a Recording Capture's MP4 bytes the same way — raw binary IPC
/// response, Capture id in, backend-derived file out, `AppError::NotFound`
/// for both a nonexistent Capture and one with no video.
#[tauri::command]
pub fn get_recording_media(
    id: String,
    db: State<DbService>,
    media: State<MediaStorage>,
) -> Result<tauri::ipc::Response, AppError> {
    let bytes = service(&db, &media).get_recording_media(&id)?;
    Ok(tauri::ipc::Response::new(bytes))
}

#[tauri::command]
pub fn update_capture(
    input: UpdateCaptureInput,
    db: State<DbService>,
    media: State<MediaStorage>,
) -> Result<Capture, AppError> {
    service(&db, &media).update(&input.id, &input.capture_type, &input.title, input.description.as_deref())
}

#[tauri::command]
pub fn delete_capture(id: String, db: State<DbService>, media: State<MediaStorage>) -> Result<(), AppError> {
    service(&db, &media).delete(&id)
}
