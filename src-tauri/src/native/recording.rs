//! Native screen recording (TASK-013). Isolated behind the
//! `RecordingEngine`/`RecordingHandle` traits — the same
//! inject-a-trait-object shape `native::screenshot`'s `ScreenshotEngine`
//! already established — so `CaptureService`/`commands::recording` never
//! touch `windows_capture` directly, and tests can inject a fake engine
//! instead of exercising the real Windows Graphics Capture API.
//!
//! Unlike a screenshot (one synchronous "capture now" call), a recording
//! is inherently two-phase: `start` returns immediately with a handle;
//! the actual capture happens on a background thread owned by the
//! `windows_capture` crate until `RecordingHandle::stop` is called.

use crate::errors::AppError;
use std::path::Path;
use windows_capture::capture::{Context, GraphicsCaptureApiHandler};
use windows_capture::encoder::{AudioSettingsBuilder, ContainerSettingsBuilder, VideoEncoder, VideoSettingsBuilder};
use windows_capture::frame::Frame;
use windows_capture::graphics_capture_api::InternalCaptureControl;
use windows_capture::monitor::Monitor;
use windows_capture::settings::{
    ColorFormat, CursorCaptureSettings, DirtyRegionSettings, DrawBorderSettings,
    MinimumUpdateIntervalSettings, SecondaryWindowSettings, Settings,
};

/// Starts a screen recording to a file. TASK-013 deliberately supports
/// only "record the primary display" — no monitor picker, no window
/// picker, no area selection — the same scope `ScreenshotEngine`
/// established for single-frame capture.
pub trait RecordingEngine: Send + Sync {
    /// Starts recording the primary display to `output_path` (the
    /// parent directory must already exist — `media::MediaStorage`
    /// guarantees this). Returns immediately; the real capture happens
    /// on a background thread until the returned handle is stopped.
    fn start(&self, output_path: &Path) -> Result<Box<dyn RecordingHandle>, AppError>;
}

/// An in-progress recording. `stop` can only be called once (it
/// consumes the handle) and blocks until the video file is fully
/// finalized and closed on disk.
pub trait RecordingHandle: Send {
    fn stop(self: Box<Self>) -> Result<(), AppError>;
}

/// The only implementation today. Uses the `windows-capture` crate
/// (Windows Graphics Capture API + a hardware-accelerated Media
/// Foundation video encoder) rather than pairing `xcap`'s own raw-frame
/// `video_recorder()` (still "WIP" upstream at the time this was
/// written) with a separately-chosen encoding crate — see DECISIONS.md
/// for the full comparison. No external runtime (no bundled ffmpeg
/// binary) is required: encoding is done by Windows itself.
pub struct WindowsRecordingEngine;

impl RecordingEngine for WindowsRecordingEngine {
    fn start(&self, output_path: &Path) -> Result<Box<dyn RecordingHandle>, AppError> {
        let monitor = Monitor::primary().map_err(|err| {
            eprintln!("[golive] failed to resolve the primary monitor for recording: {err}");
            AppError::Capture("No display is available to record.".to_string())
        })?;
        let width = monitor.width().map_err(|err| {
            eprintln!("[golive] failed to read the primary monitor's width: {err}");
            AppError::Capture("No display is available to record.".to_string())
        })?;
        let height = monitor.height().map_err(|err| {
            eprintln!("[golive] failed to read the primary monitor's height: {err}");
            AppError::Capture("No display is available to record.".to_string())
        })?;

        let settings = Settings::new(
            monitor,
            CursorCaptureSettings::Default,
            DrawBorderSettings::Default,
            SecondaryWindowSettings::Default,
            MinimumUpdateIntervalSettings::Default,
            DirtyRegionSettings::Default,
            ColorFormat::Rgba8,
            RecordingFlags { output_path: output_path.to_path_buf(), width, height },
        );

        let control = RecordingHandlerImpl::start_free_threaded(settings).map_err(|err| {
            eprintln!("[golive] failed to start screen recording: {err}");
            AppError::Capture("Failed to start screen recording.".to_string())
        })?;

        Ok(Box::new(WindowsRecordingHandle(control)))
    }
}

/// Flags passed through to `RecordingHandlerImpl::new` (see
/// `GraphicsCaptureApiHandler::Flags`) — the encoder needs the target
/// file path and frame dimensions before the first frame arrives.
struct RecordingFlags {
    output_path: std::path::PathBuf,
    width: u32,
    height: u32,
}

/// The `GraphicsCaptureApiHandler` implementation `windows_capture`
/// drives internally on its own background thread. Not exposed outside
/// this module — callers only ever see the `RecordingEngine`/
/// `RecordingHandle` trait objects above.
struct RecordingHandlerImpl {
    encoder: Option<VideoEncoder>,
}

impl GraphicsCaptureApiHandler for RecordingHandlerImpl {
    type Flags = RecordingFlags;
    type Error = Box<dyn std::error::Error + Send + Sync>;

    fn new(ctx: Context<Self::Flags>) -> Result<Self, Self::Error> {
        let encoder = VideoEncoder::new(
            VideoSettingsBuilder::new(ctx.flags.width, ctx.flags.height),
            AudioSettingsBuilder::default().disabled(true),
            ContainerSettingsBuilder::default(),
            &ctx.flags.output_path,
        )?;
        Ok(Self { encoder: Some(encoder) })
    }

    fn on_frame_arrived(
        &mut self,
        frame: &mut Frame,
        _capture_control: InternalCaptureControl,
    ) -> Result<(), Self::Error> {
        if let Some(encoder) = self.encoder.as_mut() {
            encoder.send_frame(frame)?;
        }
        Ok(())
    }

    fn on_closed(&mut self) -> Result<(), Self::Error> {
        if let Some(encoder) = self.encoder.take() {
            encoder.finish()?;
        }
        Ok(())
    }
}

/// Wraps `windows_capture`'s own `CaptureControl` — `stop()` "gracefully
/// requests the capture thread to stop and waits for it to finish",
/// which is exactly `RecordingHandle::stop`'s contract.
struct WindowsRecordingHandle(
    windows_capture::capture::CaptureControl<RecordingHandlerImpl, Box<dyn std::error::Error + Send + Sync>>,
);

impl RecordingHandle for WindowsRecordingHandle {
    fn stop(self: Box<Self>) -> Result<(), AppError> {
        self.0.stop().map_err(|err| {
            eprintln!("[golive] failed to stop/finalize screen recording: {err}");
            AppError::Capture("Failed to finish the recording.".to_string())
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Real native screen recording can't be exercised deterministically
    /// in an automated/headless environment — same limitation
    /// `native::screenshot`'s smoke test documents. `#[ignore]`d so
    /// `cargo test` never depends on an interactive desktop session; run
    /// manually (`cargo test --release -- --ignored
    /// record_primary_display_smoke_test`) on a real Windows desktop.
    #[test]
    #[ignore]
    fn record_primary_display_smoke_test() {
        let dir = tempfile::tempdir().expect("temp dir");
        let output_path = dir.path().join("smoke-test.mp4");

        let handle = WindowsRecordingEngine.start(&output_path).expect("recording should start");
        std::thread::sleep(std::time::Duration::from_secs(2));
        handle.stop().expect("recording should stop cleanly");

        let metadata = std::fs::metadata(&output_path).expect("output file should exist");
        assert!(metadata.len() > 0, "output file should not be empty");
    }
}
