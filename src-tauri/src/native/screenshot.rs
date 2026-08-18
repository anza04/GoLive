//! Native screen capture. Isolated behind the `ScreenshotEngine` trait so
//! `CaptureService` (and the frontend, further up) never touches `xcap` or
//! any Windows-specific capture detail directly — the same
//! inject-a-trait-object shape already used for repositories (see
//! docs/architecture.md, "Persistence boundary"). This is also the seam a
//! later task would extend for monitor selection, area selection, or a
//! screen-recording engine, without CaptureService or React changing.

use crate::errors::AppError;
use std::io::Cursor;
use xcap::image::ImageFormat;
use xcap::Monitor;

/// Captures a Windows display and returns it PNG-encoded. Nothing is
/// written to disk here — persisting the bytes is `media::MediaStorage`'s
/// job (see docs/architecture.md, "Media storage boundary"); this trait
/// only knows how to talk to the OS/display.
pub trait ScreenshotEngine: Send + Sync {
    /// Captures the current primary display and returns PNG-encoded
    /// bytes. TASK-009 deliberately supports only this one mode (see
    /// docs/architecture.md) — no monitor picker, no area/window
    /// selection yet.
    fn capture_primary_display(&self) -> Result<Vec<u8>, AppError>;
}

/// The only implementation today. Uses the cross-platform `xcap` crate
/// (GDI-based on Windows — the simplest reliable backend; `xcap`'s
/// optional Windows-Graphics-Capture backend was not enabled, see
/// DECISIONS.md) to grab the primary display and encodes it as PNG in
/// memory. GoLive's only supported target platform is Windows (see
/// docs/architecture.md §1), so no other platform branch exists here.
pub struct WindowsScreenshotEngine;

impl ScreenshotEngine for WindowsScreenshotEngine {
    fn capture_primary_display(&self) -> Result<Vec<u8>, AppError> {
        let monitors = Monitor::all()?;
        let monitor = monitors
            .into_iter()
            .find(|monitor| monitor.is_primary().unwrap_or(false))
            .ok_or_else(|| AppError::Capture("No display is available to capture.".to_string()))?;

        let image = monitor.capture_image()?;

        let mut png_bytes = Vec::new();
        image
            .write_to(&mut Cursor::new(&mut png_bytes), ImageFormat::Png)
            .map_err(|err| {
                eprintln!("[golive] failed to encode screenshot as PNG: {err}");
                AppError::Capture("Failed to encode the screenshot.".to_string())
            })?;

        Ok(png_bytes)
    }
}

impl From<xcap::XCapError> for AppError {
    fn from(err: xcap::XCapError) -> Self {
        eprintln!("[golive] screenshot capture error: {err}");
        AppError::Capture("Screenshot capture failed. Please try again.".to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Real native screen capture can't be exercised deterministically in
    /// an automated/headless environment (see docs/architecture.md,
    /// "Screenshot capture testing limitation") — this build/CI agent may
    /// have no interactive desktop session at all, which `xcap` needs.
    /// `#[ignore]`d so `cargo test` never depends on one; run manually
    /// (`cargo test -- --ignored capture_primary_display_smoke_test`) on
    /// a real interactive Windows desktop to sanity-check the engine
    /// directly, on top of the manual `golive.exe` validation this task
    /// otherwise relies on (see PROJECT_STATE.md).
    #[test]
    #[ignore]
    fn capture_primary_display_smoke_test() {
        let png_bytes = WindowsScreenshotEngine.capture_primary_display().expect("capture should succeed");
        assert!(!png_bytes.is_empty());
        assert_eq!(&png_bytes[0..8], b"\x89PNG\r\n\x1a\n", "output should be a valid PNG");
    }
}
