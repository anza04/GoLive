//! In-progress screen-recording state (TASK-013, extended TASK-014): a
//! small piece of Tauri-managed state bridging the two-phase
//! `start_recording_capture`/`stop_recording_capture` commands, and
//! (TASK-014) the cross-window "is a recording in progress, and for how
//! long" mirror the floating widget and the main window's Captures
//! section both read — the same cross-window problem
//! `active_process::ActiveProcessState` already solved for "which
//! Process is active" (see docs/architecture.md).
//!
//! `CaptureService` is deliberately stateless — a fresh instance is
//! built for every command (see `commands::capture::service` and
//! `commands::recording::capture_service`), the same "construct fresh,
//! backed by shared state" shape `db::DbPool`/`media::MediaStorage`
//! already use. A recording's in-progress `RecordingHandle`, however,
//! has to survive between the `start` and `stop` commands — two
//! completely separate IPC calls — so it lives here instead.
//!
//! Deliberately supports **at most one recording at a time, system-wide**
//! — no per-Process/per-window keying, no queue. See DECISIONS.md for
//! why this is the right scope rather than a limitation to work around.

use crate::errors::AppError;
use crate::native::recording::RecordingHandle;
use serde::Serialize;
use std::sync::Mutex;

/// The read-only, cloneable "is a recording in progress, and which one"
/// shape — everything a window needs to render a Start/Stop control and
/// an elapsed-time indicator, but not the live native handle itself
/// (which can't be cloned). This is both `start_recording_capture`'s
/// return value and `recording-status-changed`'s event payload (see
/// `commands::recording`) — the same "one struct, both the state and the
/// wire shape" pattern `active_process::ActiveProcessInfo` already uses.
#[derive(Debug, Clone, Serialize)]
pub struct RecordingStatusInfo {
    pub id: String,
    pub process_id: String,
    pub title: String,
    /// Unix epoch milliseconds (UTC) — when the recording started.
    /// Elapsed-time display is computed client-side (`Date.now() -
    /// startedAt`, ticked with `setInterval`); the backend never sends a
    /// pre-formatted duration string, the same "timestamps are raw, the
    /// frontend formats" rule every other display date in the app
    /// follows (see `utils/formatDate`).
    pub started_at: i64,
}

/// Everything needed to finalize a recording once it's stopped: the
/// cloneable status above (id, process, title, start time), the
/// already-validated/trimmed description carried forward from
/// `start_recording_capture`, and the live native handle.
pub struct InProgressRecording {
    pub status: RecordingStatusInfo,
    pub description: Option<String>,
    pub handle: Box<dyn RecordingHandle>,
}

#[derive(Default)]
pub struct RecordingState(Mutex<Option<InProgressRecording>>);

impl RecordingState {
    /// Atomically checks "is a recording already in progress?" and, if
    /// not, runs `start` (which does the actual validation and native
    /// `RecordingEngine::start` call) and stores its result — all while
    /// holding the lock, so two concurrent `start_recording_capture`
    /// calls can never both pass the "nothing in progress" check and
    /// both start a real native recording (only one would ever be
    /// stored, silently leaking the other's capture thread). Returns a
    /// clone of the stored status for the command to build its response
    /// (and broadcast event) from.
    pub fn start<F>(&self, start: F) -> Result<RecordingStatusInfo, AppError>
    where
        F: FnOnce() -> Result<InProgressRecording, AppError>,
    {
        let mut guard = self.0.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
        if guard.is_some() {
            return Err(AppError::Validation(
                "A recording is already in progress. Stop it before starting another.".to_string(),
            ));
        }

        let recording = start()?;
        let status = recording.status.clone();
        *guard = Some(recording);
        Ok(status)
    }

    /// Takes the in-progress recording, if any, leaving `None` behind —
    /// `stop_recording_capture`'s first step. A second call (nothing
    /// left to take) returns `None`, which the command turns into a
    /// clear "no recording is in progress" `AppError::Validation`.
    pub fn take(&self) -> Option<InProgressRecording> {
        let mut guard = self.0.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
        guard.take()
    }

    /// Reads the current status without consuming anything — lets a
    /// window that just opened (or wasn't listening for the last
    /// `recording-status-changed` event) fetch "is a recording in
    /// progress right now" on mount, the same role
    /// `active_process::ActiveProcessState::get` plays for the active
    /// Process.
    pub fn status(&self) -> Option<RecordingStatusInfo> {
        let guard = self.0.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
        guard.as_ref().map(|recording| recording.status.clone())
    }
}

/// Emitted (via `AppHandle::emit`, Rust-side — not gated by the frontend
/// ACL) to every window whenever a recording starts or stops. Payload is
/// `Option<RecordingStatusInfo>` — `null` means no recording is in
/// progress.
pub const RECORDING_STATUS_CHANGED_EVENT: &str = "recording-status-changed";

#[cfg(test)]
mod tests {
    use super::*;

    struct FakeHandle {
        stopped: std::sync::Arc<std::sync::atomic::AtomicBool>,
    }

    impl RecordingHandle for FakeHandle {
        fn stop(self: Box<Self>) -> Result<(), AppError> {
            self.stopped.store(true, std::sync::atomic::Ordering::SeqCst);
            Ok(())
        }
    }

    fn sample_recording() -> InProgressRecording {
        InProgressRecording {
            status: RecordingStatusInfo {
                id: "capture-1".to_string(),
                process_id: "process-1".to_string(),
                title: "My recording".to_string(),
                started_at: 1000,
            },
            description: None,
            handle: Box::new(FakeHandle { stopped: std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false)) }),
        }
    }

    #[test]
    fn start_stores_the_recording_and_returns_its_status() {
        let state = RecordingState::default();
        let status = state.start(|| Ok(sample_recording())).expect("start");

        assert_eq!(status.id, "capture-1");
        assert_eq!(status.process_id, "process-1");
        assert_eq!(status.title, "My recording");
        assert_eq!(status.started_at, 1000);
    }

    #[test]
    fn start_rejects_a_second_recording_while_one_is_in_progress() {
        let state = RecordingState::default();
        state.start(|| Ok(sample_recording())).expect("first start");

        let result = state.start(|| Ok(sample_recording()));
        assert!(matches!(result, Err(AppError::Validation(_))));
    }

    #[test]
    fn start_propagates_a_failure_from_the_closure_without_storing_anything() {
        let state = RecordingState::default();
        let result: Result<_, AppError> =
            state.start(|| Err(AppError::Validation("nope".to_string())));
        assert!(result.is_err());

        // Nothing was stored, so a subsequent start must succeed.
        state.start(|| Ok(sample_recording())).expect("start after a failed attempt should succeed");
    }

    #[test]
    fn take_returns_the_recording_once_and_none_afterward() {
        let state = RecordingState::default();
        state.start(|| Ok(sample_recording())).expect("start");

        let taken = state.take();
        assert!(taken.is_some());
        assert_eq!(taken.unwrap().status.id, "capture-1");

        assert!(state.take().is_none(), "a second take must find nothing left");
    }

    #[test]
    fn after_take_a_new_recording_can_be_started() {
        let state = RecordingState::default();
        state.start(|| Ok(sample_recording())).expect("start");
        state.take();

        state.start(|| Ok(sample_recording())).expect("start after take should succeed");
    }

    #[test]
    fn status_reflects_the_current_in_progress_recording() {
        let state = RecordingState::default();
        assert!(state.status().is_none(), "nothing in progress yet");

        state.start(|| Ok(sample_recording())).expect("start");
        let status = state.status().expect("a recording is in progress");
        assert_eq!(status.id, "capture-1");

        state.take();
        assert!(state.status().is_none(), "nothing left after take");
    }
}
