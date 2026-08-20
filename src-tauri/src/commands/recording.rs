//! Thin Tauri-facing commands for the two-phase screen-recording flow
//! (TASK-013, extended TASK-014). `start_recording_capture` validates
//! the request and starts a real native recording
//! (`native::recording::WindowsRecordingEngine`), storing its
//! in-progress handle in the Tauri-managed `RecordingState` (see
//! `recording.rs`) rather than in `CaptureService`, which is
//! deliberately stateless/rebuilt fresh per command (see
//! docs/architecture.md). `stop_recording_capture` retrieves that
//! handle, finalizes the video file, and creates the Capture metadata
//! row — mirroring `commands::capture::create_screenshot_capture`'s
//! transactional-create discipline, just split across two commands
//! since a recording can't be captured in one synchronous call.
//! `get_recording_status` and the `recording-status-changed` event
//! broadcast (TASK-014) let every window — the main window's Captures
//! section and the floating widget alike — show "is a recording in
//! progress, and for how long" regardless of which window started it,
//! the same cross-window problem `commands::active_process` already
//! solved for the active Process.

use crate::db::DbService;
use crate::errors::AppError;
use crate::media::MediaStorage;
use crate::models::capture::Capture;
use crate::native::recording::{RecordingEngine, WindowsRecordingEngine};
use crate::native::screenshot::WindowsScreenshotEngine;
use crate::recording::{InProgressRecording, RecordingState, RecordingStatusInfo, RECORDING_STATUS_CHANGED_EVENT};
use crate::repositories::capture::SqliteCaptureRepository;
use crate::repositories::process::SqliteProcessRepository;
use crate::services::capture::CaptureService;
use serde::Deserialize;
use tauri::{AppHandle, Emitter, State};

#[derive(Deserialize)]
pub struct StartRecordingInput {
    pub process_id: String,
    pub title: String,
    #[serde(default)]
    pub description: Option<String>,
    /// Opt-in microphone audio (TASK-015) — defaults to `false` so any
    /// caller that predates this field (there are none left in this
    /// codebase, but the wire format shouldn't require it) still gets
    /// TASK-013/014's original video-only behavior.
    #[serde(default)]
    pub include_audio: bool,
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
/// `recording::RecordingState`). Broadcasts `recording-status-changed`
/// on success so every window reflects the new in-progress recording
/// immediately.
#[tauri::command]
pub fn start_recording_capture(
    input: StartRecordingInput,
    db: State<DbService>,
    media: State<MediaStorage>,
    recording: State<RecordingState>,
    app: AppHandle,
) -> Result<RecordingStatusInfo, AppError> {
    let service = capture_service(&db, &media);
    let media_storage = media.inner().clone();

    let status = recording.start(move || {
        let (title, description) =
            service.validate_recording_start(&input.process_id, &input.title, input.description.as_deref())?;

        let id = uuid::Uuid::new_v4().to_string();
        let output_path = media_storage.video_path(&id)?;
        let handle = WindowsRecordingEngine.start(&output_path, input.include_audio)?;

        Ok(InProgressRecording {
            status: RecordingStatusInfo { id, process_id: input.process_id, title, started_at: now_ms() },
            description: if description.is_empty() { None } else { Some(description) },
            handle,
        })
    })?;

    if let Err(err) = app.emit(RECORDING_STATUS_CHANGED_EVENT, Some(&status)) {
        eprintln!("[golive] failed to broadcast recording start: {err}");
    }

    Ok(status)
}

/// Stops the in-progress recording (blocking until the video file is
/// fully finalized on disk) and creates its Capture metadata row. Fails
/// with `AppError::Validation` if no recording is currently in progress.
#[tauri::command]
pub fn stop_recording_capture(
    db: State<DbService>,
    media: State<MediaStorage>,
    recording: State<RecordingState>,
    app: AppHandle,
) -> Result<Capture, AppError> {
    let in_progress = recording
        .take()
        .ok_or_else(|| AppError::Validation("No recording is in progress.".to_string()))?;

    // `take()` already cleared `RecordingState` — broadcast that now,
    // before attempting to stop/finalize below, so no window is left
    // showing a ticking "recording in progress" indicator for a
    // recording that isn't running anymore even if finalizing fails.
    if let Err(err) = app.emit(RECORDING_STATUS_CHANGED_EVENT, Option::<RecordingStatusInfo>::None) {
        eprintln!("[golive] failed to broadcast recording stop: {err}");
    }

    in_progress.handle.stop()?;

    let capture = capture_service(&db, &media).finalize_recording(
        &in_progress.status.id,
        &in_progress.status.process_id,
        &in_progress.status.title,
        in_progress.description.as_deref().unwrap_or(""),
    )?;

    // Same broadcast `create_capture`/`create_screenshot_capture` send
    // (see `commands::capture::CAPTURE_CREATED_EVENT`) — a recording
    // stopped from the widget needs the main window's already-open
    // Captures section to pick it up immediately too, not just clear its
    // in-progress indicator.
    if let Err(err) = app.emit(crate::commands::capture::CAPTURE_CREATED_EVENT, &capture) {
        eprintln!("[golive] failed to broadcast capture creation: {err}");
    }

    Ok(capture)
}

/// Lets a window that wasn't open (or wasn't listening yet) when a
/// recording started/stopped — e.g. the Captures section, just switched
/// to a different Process — fetch the current status on mount instead
/// of waiting for the next `recording-status-changed` event.
#[tauri::command]
pub fn get_recording_status(recording: State<RecordingState>) -> Option<RecordingStatusInfo> {
    recording.status()
}

fn now_ms() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis() as i64)
        .unwrap_or(0)
}
