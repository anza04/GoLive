//! In-progress screen-recording state (TASK-013): a small piece of
//! Tauri-managed state bridging the two-phase `start_recording_capture`/
//! `stop_recording_capture` commands (see `commands::recording`).
//!
//! `CaptureService` is deliberately stateless — a fresh instance is
//! built for every command (see `commands::capture::service` and
//! `commands::recording::capture_service`), the same "construct fresh,
//! backed by shared state" shape `db::DbPool`/`media::MediaStorage`
//! already use. A recording's in-progress `RecordingHandle`, however,
//! has to survive between the `start` and `stop` commands — two
//! completely separate IPC calls — so it lives here instead, mirroring
//! `active_process::ActiveProcessState`'s shape (a small
//! `Mutex`-protected value managed via `app.manage(...)`).
//!
//! Deliberately supports **at most one recording at a time, system-wide**
//! — no per-Process/per-window keying, no queue. See DECISIONS.md for
//! why this is the right scope for TASK-013 rather than a limitation to
//! work around.

use crate::errors::AppError;
use crate::native::recording::RecordingHandle;
use std::sync::Mutex;

/// Everything needed to finalize a recording once it's stopped: the
/// pre-generated Capture id the video file was written under (see
/// `media::MediaStorage::video_path`), the owning process, the
/// already-validated/trimmed title/description carried forward from
/// `start_recording_capture`, and the live native handle.
pub struct InProgressRecording {
    pub id: String,
    pub process_id: String,
    pub title: String,
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
    /// stored, silently leaking the other's capture thread). Returns the
    /// `(id, process_id, title)` of the recording that was started, for
    /// the command to build its response from.
    pub fn start<F>(&self, start: F) -> Result<(String, String, String), AppError>
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
        let info = (recording.id.clone(), recording.process_id.clone(), recording.title.clone());
        *guard = Some(recording);
        Ok(info)
    }

    /// Takes the in-progress recording, if any, leaving `None` behind —
    /// `stop_recording_capture`'s first step. A second call (nothing
    /// left to take) returns `None`, which the command turns into a
    /// clear "no recording is in progress" `AppError::Validation`.
    pub fn take(&self) -> Option<InProgressRecording> {
        let mut guard = self.0.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
        guard.take()
    }
}

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
            id: "capture-1".to_string(),
            process_id: "process-1".to_string(),
            title: "My recording".to_string(),
            description: None,
            handle: Box::new(FakeHandle { stopped: std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false)) }),
        }
    }

    #[test]
    fn start_stores_the_recording_and_returns_its_info() {
        let state = RecordingState::default();
        let (id, process_id, title) = state.start(|| Ok(sample_recording())).expect("start");

        assert_eq!(id, "capture-1");
        assert_eq!(process_id, "process-1");
        assert_eq!(title, "My recording");
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
        assert_eq!(taken.unwrap().id, "capture-1");

        assert!(state.take().is_none(), "a second take must find nothing left");
    }

    #[test]
    fn after_take_a_new_recording_can_be_started() {
        let state = RecordingState::default();
        state.start(|| Ok(sample_recording())).expect("start");
        state.take();

        state.start(|| Ok(sample_recording())).expect("start after take should succeed");
    }
}
