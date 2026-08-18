//! Thin Tauri-facing commands for TASK-013's two-phase screen-recording
//! flow. `start_recording_capture` validates the request and starts a
//! real native recording (`native::recording::WindowsRecordingEngine`),
//! storing its in-progress handle in the Tauri-managed `RecordingState`
//! (see `recording.rs`) rather than in `CaptureService`, which is
//! deliberately stateless/rebuilt fresh per command (see
//! docs/architecture.md). `stop_recording_capture` retrieves that
//! handle, finalizes the video file, and creates the Capture metadata
//! row — mirroring `commands::capture::create_screenshot_capture`'s
//! transactional-create discipline, just split across two commands
//! since a recording can't be captured in one synchronous call.

use crate::db::DbService;
use crate::errors::AppError;
use crate::media::MediaStorage;
use crate::models::capture::Capture;
use crate::native::recording::{RecordingEngine, WindowsRecordingEngine};
use crate::native::screenshot::WindowsScreenshotEngine;
use crate::recording::{InProgressRecording, RecordingState};
use crate::repositories::capture::SqliteCaptureRepository;
use crate::repositories::process::SqliteProcessRepository;
use crate::services::capture::CaptureService;
use serde::{Deserialize, Serialize};
use tauri::State;

#[derive(Deserialize)]
pub struct StartRecordingInput {
    pub process_id: String,
    pub title: String,
    #[serde(default)]
    pub description: Option<String>,
}

/// What `start_recording_capture` returns — an in-progress marker, not a
/// full `Capture`: no metadata row exists yet (see `recording.rs`'s
/// module doc and DECISIONS.md). The frontend doesn't currently do
/// anything with this beyond knowing the start succeeded, but returning
/// it (rather than `()`) keeps the command symmetrical with every other
/// create-style command, which all return the thing they just made.
#[derive(Serialize)]
pub struct RecordingStartedInfo {
    pub id: String,
    pub process_id: String,
    pub title: String,
}

/// Same construction shape as `commands::capture::service` — a fresh
/// `CaptureService` built from the managed `DbService`/`MediaStorage`
/// for each command. Recording commands need the same
/// validation/finalization logic `CaptureService` already owns, even
/// though they don't call any of its screenshot-specific methods; a
/// `WindowsScreenshotEngine` is still supplied because `CaptureService::new`
/// takes one unconditionally (see docs/architecture.md) — the same
/// three-times-repeated construction DECISIONS.md already accepted for
/// `hotkey::handle_capture_shortcut`.
fn capture_service(db: &State<DbService>, media: &State<MediaStorage>) -> CaptureService {
    CaptureService::new(
        Box::new(SqliteCaptureRepository::new(db.pool())),
        Box::new(SqliteProcessRepository::new(db.pool())),
        media.inner().clone(),
        Box::new(WindowsScreenshotEngine),
    )
}

/// Validates the request, then starts a real recording of the primary
/// display to `<capture id>.mp4` (see `media::MediaStorage::video_path`).
/// No Capture metadata row exists yet — that's created by
/// `stop_recording_capture` once the video is finalized. Fails with
/// `AppError::Validation` if a recording is already in progress (see
/// `recording::RecordingState`).
#[tauri::command]
pub fn start_recording_capture(
    input: StartRecordingInput,
    db: State<DbService>,
    media: State<MediaStorage>,
    recording: State<RecordingState>,
) -> Result<RecordingStartedInfo, AppError> {
    let service = capture_service(&db, &media);
    let media_storage = media.inner().clone();

    let (id, process_id, title) = recording.start(move || {
        let (title, description) =
            service.validate_recording_start(&input.process_id, &input.title, input.description.as_deref())?;

        let id = uuid::Uuid::new_v4().to_string();
        let output_path = media_storage.video_path(&id)?;
        let handle = WindowsRecordingEngine.start(&output_path)?;

        Ok(InProgressRecording {
            id,
            process_id: input.process_id,
            title,
            description: if description.is_empty() { None } else { Some(description) },
            handle,
        })
    })?;

    Ok(RecordingStartedInfo { id, process_id, title })
}

/// Stops the in-progress recording (blocking until the video file is
/// fully finalized on disk) and creates its Capture metadata row. Fails
/// with `AppError::Validation` if no recording is currently in progress.
#[tauri::command]
pub fn stop_recording_capture(
    db: State<DbService>,
    media: State<MediaStorage>,
    recording: State<RecordingState>,
) -> Result<Capture, AppError> {
    let in_progress = recording
        .take()
        .ok_or_else(|| AppError::Validation("No recording is in progress.".to_string()))?;

    in_progress.handle.stop()?;

    capture_service(&db, &media).finalize_recording(
        &in_progress.id,
        &in_progress.process_id,
        &in_progress.title,
        in_progress.description.as_deref().unwrap_or(""),
    )
}
