//! Native screen recording (TASK-013), extended with optional
//! microphone audio (TASK-015). Isolated behind the
//! `RecordingEngine`/`RecordingHandle` traits — the same
//! inject-a-trait-object shape `native::screenshot`'s `ScreenshotEngine`
//! already established — so `CaptureService`/`commands::recording` never
//! touch `windows_capture`/`cpal` directly, and tests can inject a fake
//! engine instead of exercising the real Windows Graphics Capture API.
//!
//! Unlike a screenshot (one synchronous "capture now" call), a recording
//! is inherently two-phase: `start` returns immediately with a handle;
//! the actual capture happens on a background thread owned by the
//! `windows_capture` crate until `RecordingHandle::stop` is called.

use crate::errors::AppError;
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use std::path::Path;
use std::sync::{Arc, Mutex};
use windows_capture::capture::{Context, GraphicsCaptureApiHandler};
use windows_capture::encoder::{
    AudioSettingsBuilder, ContainerSettingsBuilder, VideoEncoder, VideoSettingsBuilder, VideoSettingsSubType,
};
use windows_capture::frame::Frame;
use windows_capture::graphics_capture_api::InternalCaptureControl;
use windows_capture::monitor::Monitor;
use windows_capture::settings::{
    ColorFormat, CursorCaptureSettings, DirtyRegionSettings, DrawBorderSettings,
    MinimumUpdateIntervalSettings, SecondaryWindowSettings, Settings,
};

/// Starts a screen recording to a file, optionally muxing in the
/// default microphone (TASK-015). TASK-013 deliberately supports only
/// "record the primary display" — no monitor picker, no window picker,
/// no area selection — the same scope `ScreenshotEngine` established
/// for single-frame capture; that scope is unchanged here.
pub trait RecordingEngine: Send + Sync {
    /// Starts recording the primary display to `output_path` (the
    /// parent directory must already exist — `media::MediaStorage`
    /// guarantees this), optionally including the system's default
    /// microphone if `include_audio` is true. Returns immediately; the
    /// real capture happens on a background thread until the returned
    /// handle is stopped. If `include_audio` is true and no microphone
    /// is available, this fails rather than silently recording
    /// video-only — the caller asked for audio.
    fn start(&self, output_path: &Path, include_audio: bool) -> Result<Box<dyn RecordingHandle>, AppError>;
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
/// binary) is required: encoding is done by Windows itself. Microphone
/// *capture* (TASK-015) is a genuinely separate concern from screen
/// *capture* — `windows_capture`'s own audio support is encode-only (you
/// feed it PCM bytes, it doesn't record a microphone for you) — so
/// `cpal` (the standard cross-platform Rust audio I/O crate, WASAPI-
/// backed on Windows with no extra Cargo feature needed) supplies the
/// actual microphone samples; see DECISIONS.md for why it was chosen
/// over hand-rolling WASAPI bindings directly.
pub struct WindowsRecordingEngine;

impl RecordingEngine for WindowsRecordingEngine {
    fn start(&self, output_path: &Path, include_audio: bool) -> Result<Box<dyn RecordingHandle>, AppError> {
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
            RecordingFlags { output_path: output_path.to_path_buf(), width, height, include_audio },
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
/// file path and frame dimensions before the first frame arrives, and
/// (TASK-015) whether to also open the default microphone.
struct RecordingFlags {
    output_path: std::path::PathBuf,
    width: u32,
    height: u32,
    include_audio: bool,
}

/// The `GraphicsCaptureApiHandler` implementation `windows_capture`
/// drives internally on its own background thread. Not exposed outside
/// this module — callers only ever see the `RecordingEngine`/
/// `RecordingHandle` trait objects above.
///
/// `encoder` is `Arc<Mutex<Option<VideoEncoder>>>`, not a plain
/// `Option<VideoEncoder>`, because (TASK-015) it's written to from two
/// genuinely different threads: `on_frame_arrived` (called by
/// `windows_capture`'s own capture thread) and the microphone's audio
/// callback (called by `cpal`'s own audio thread, set up in `new` and
/// kept alive by `_audio_stream`). The `Option` (rather than requiring
/// callers to have a valid `VideoEncoder` at all times) is what lets
/// `on_closed` `.take()` it out of the shared slot to call the
/// consuming `VideoEncoder::finish()` — the crate expects an owned
/// `Self`, not `&mut`.
struct RecordingHandlerImpl {
    encoder: Arc<Mutex<Option<VideoEncoder>>>,
    /// Never read after construction — its only job is to keep the OS
    /// microphone capture running for as long as it's alive. Dropped
    /// (stopping the microphone) in `on_closed`, before the encoder is
    /// finalized, so no audio callback can fire mid-`finish()`.
    _audio_stream: Option<cpal::Stream>,
}

impl GraphicsCaptureApiHandler for RecordingHandlerImpl {
    type Flags = RecordingFlags;
    type Error = Box<dyn std::error::Error + Send + Sync>;

    fn new(ctx: Context<Self::Flags>) -> Result<Self, Self::Error> {
        let mic = if ctx.flags.include_audio { Some(default_microphone_format()?) } else { None };

        let audio_settings = match &mic {
            Some(format) => AudioSettingsBuilder::default()
                .sample_rate(format.sample_rate)
                .channel_count(format.channels)
                .bit_per_sample(16)
                .disabled(false),
            None => AudioSettingsBuilder::default().disabled(true),
        };

        let encoder = VideoEncoder::new(
            // `VideoSettingsBuilder::new` defaults to `VideoSettingsSubType::HEVC`
            // if `.sub_type(...)` isn't called — H.265 in an MP4 container,
            // which Chromium/WebView2 (GoLive's playback surface,
            // `CaptureDetail`'s `<video>` element, TASK-014) does **not**
            // decode without extra OS codec packs, so a recording made
            // with the library's own default would silently fail to
            // play. H.264 is universally supported by Chromium's built-in
            // decoder, so it's set explicitly here rather than relying on
            // the crate's default (see DECISIONS.md).
            VideoSettingsBuilder::new(ctx.flags.width, ctx.flags.height).sub_type(VideoSettingsSubType::H264),
            audio_settings,
            ContainerSettingsBuilder::default(),
            &ctx.flags.output_path,
        )?;

        let encoder = Arc::new(Mutex::new(Some(encoder)));

        let audio_stream = match mic {
            Some(format) => Some(start_microphone_stream(format, encoder.clone())?),
            None => None,
        };

        Ok(Self { encoder, _audio_stream: audio_stream })
    }

    fn on_frame_arrived(
        &mut self,
        frame: &mut Frame,
        _capture_control: InternalCaptureControl,
    ) -> Result<(), Self::Error> {
        let mut guard = self.encoder.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
        if let Some(encoder) = guard.as_mut() {
            encoder.send_frame(frame)?;
        }
        Ok(())
    }

    fn on_closed(&mut self) -> Result<(), Self::Error> {
        // Stop the microphone first — once this drops, `cpal` guarantees
        // no further audio callbacks fire, so `finish()` below can't
        // race a callback still trying to write into the encoder it's
        // about to consume.
        self._audio_stream = None;

        let mut guard = self.encoder.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
        if let Some(encoder) = guard.take() {
            encoder.finish()?;
        }
        Ok(())
    }
}

/// The microphone format `windows_capture`'s `AudioSettingsBuilder` is
/// configured to expect, and what `start_microphone_stream` actually
/// opens the device with — always the OS's own reported default input
/// format (see docs/architecture.md), never resampled: whatever
/// `sample_rate`/`channels` the device's default config reports is what
/// the encoder is told to expect, so there's no format-mismatch risk
/// between what's captured and what's declared.
struct MicrophoneFormat {
    sample_rate: u32,
    channels: u32,
}

/// Queries the default microphone's default input format. Fails with a
/// descriptive error (via `?` into `RecordingHandlerImpl::new`'s
/// `Box<dyn Error + Send + Sync>`) if no input device exists — the
/// caller asked for audio, so silently falling back to video-only would
/// be surprising, not helpful.
fn default_microphone_format() -> Result<MicrophoneFormat, Box<dyn std::error::Error + Send + Sync>> {
    let host = cpal::default_host();
    let device = host
        .default_input_device()
        .ok_or_else(|| -> Box<dyn std::error::Error + Send + Sync> { "No microphone is available.".into() })?;
    let config = device.default_input_config()?;
    Ok(MicrophoneFormat { sample_rate: config.sample_rate(), channels: config.channels() as u32 })
}

/// Opens the default microphone and starts streaming its audio straight
/// into `encoder` (via `VideoEncoder::send_audio_buffer`, which expects
/// interleaved 16-bit PCM) for as long as the returned `cpal::Stream`
/// stays alive. `cpal` delivers samples as `f32` or `i16` depending on
/// the device/driver; both are handled, converted to 16-bit PCM bytes.
/// Any other reported sample format is rejected rather than silently
/// producing garbage audio.
fn start_microphone_stream(
    format: MicrophoneFormat,
    encoder: Arc<Mutex<Option<VideoEncoder>>>,
) -> Result<cpal::Stream, Box<dyn std::error::Error + Send + Sync>> {
    let host = cpal::default_host();
    let device = host
        .default_input_device()
        .ok_or_else(|| -> Box<dyn std::error::Error + Send + Sync> { "No microphone is available.".into() })?;
    let supported_config = device.default_input_config()?;
    let sample_format = supported_config.sample_format();
    let stream_config: cpal::StreamConfig = supported_config.into();

    let stream = match sample_format {
        cpal::SampleFormat::F32 => {
            let encoder = encoder.clone();
            device.build_input_stream(
                stream_config,
                move |data: &[f32], _: &cpal::InputCallbackInfo| feed_audio(&encoder, &f32_to_pcm16(data)),
                |err: cpal::Error| eprintln!("[golive] microphone capture error: {err}"),
                None,
            )?
        }
        cpal::SampleFormat::I16 => {
            let encoder = encoder.clone();
            device.build_input_stream(
                stream_config,
                move |data: &[i16], _: &cpal::InputCallbackInfo| feed_audio(&encoder, &i16_to_pcm16(data)),
                |err: cpal::Error| eprintln!("[golive] microphone capture error: {err}"),
                None,
            )?
        }
        other => {
            return Err(format!("Unsupported microphone sample format: {other:?}").into());
        }
    };

    stream.play()?;
    let _ = format; // sample_rate/channels were only needed to configure the encoder, above.
    Ok(stream)
}

/// Feeds already-16-bit-PCM `bytes` into the shared encoder. A poisoned
/// lock or an encoder already taken (recording stopping/stopped) is a
/// silent no-op — a lagging audio callback losing its last few samples
/// during shutdown is expected and harmless, not an error worth
/// surfacing (there's nowhere to surface it to from inside `cpal`'s own
/// callback thread anyway).
fn feed_audio(encoder: &Arc<Mutex<Option<VideoEncoder>>>, bytes: &[u8]) {
    let Ok(mut guard) = encoder.lock() else { return };
    if let Some(encoder) = guard.as_mut() {
        if let Err(err) = encoder.send_audio_buffer(bytes, 0) {
            eprintln!("[golive] failed to encode a microphone audio buffer: {err}");
        }
    }
}

/// Converts interleaved `f32` samples in `[-1.0, 1.0]` to interleaved
/// 16-bit signed PCM, little-endian — the format
/// `VideoEncoder::send_audio_buffer` expects. A plain linear conversion,
/// not resampling: the encoder was already configured (`AudioSettingsBuilder`,
/// see `RecordingHandlerImpl::new`) to expect exactly this device's own
/// sample rate/channel count, so no rate/channel conversion is needed —
/// only the sample *format* changes.
fn f32_to_pcm16(samples: &[f32]) -> Vec<u8> {
    let mut bytes = Vec::with_capacity(samples.len() * 2);
    for sample in samples {
        let clamped = sample.clamp(-1.0, 1.0);
        let pcm = (clamped * i16::MAX as f32) as i16;
        bytes.extend_from_slice(&pcm.to_le_bytes());
    }
    bytes
}

/// Converts interleaved `i16` samples to interleaved 16-bit signed PCM
/// bytes, little-endian — already the right sample format, just
/// serialized to bytes.
fn i16_to_pcm16(samples: &[i16]) -> Vec<u8> {
    let mut bytes = Vec::with_capacity(samples.len() * 2);
    for sample in samples {
        bytes.extend_from_slice(&sample.to_le_bytes());
    }
    bytes
}

/// Wraps `windows_capture`'s own `CaptureControl` — `stop()` "gracefully
/// requests the capture thread to stop and waits for it to finish",
/// which is exactly `RecordingHandle::stop`'s contract. Stopping the
/// microphone (TASK-015) happens inside `RecordingHandlerImpl::on_closed`,
/// triggered by this same `stop()` call — there's nothing extra for this
/// wrapper itself to do for audio.
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

        let handle = WindowsRecordingEngine.start(&output_path, false).expect("recording should start");
        std::thread::sleep(std::time::Duration::from_secs(2));
        handle.stop().expect("recording should stop cleanly");

        let metadata = std::fs::metadata(&output_path).expect("output file should exist");
        assert!(metadata.len() > 0, "output file should not be empty");
    }

    /// Same as above but with the microphone enabled — a real,
    /// interactive-desktop-only smoke test for TASK-015's audio path.
    /// `#[ignore]`d for the same reason; also requires a real
    /// microphone/input device to be present, unlike the video-only
    /// test above.
    #[test]
    #[ignore]
    fn record_primary_display_with_audio_smoke_test() {
        let dir = tempfile::tempdir().expect("temp dir");
        let output_path = dir.path().join("smoke-test-audio.mp4");

        let handle = WindowsRecordingEngine.start(&output_path, true).expect("recording with audio should start");
        std::thread::sleep(std::time::Duration::from_secs(3));
        handle.stop().expect("recording should stop cleanly");

        let metadata = std::fs::metadata(&output_path).expect("output file should exist");
        assert!(metadata.len() > 0, "output file should not be empty");
    }
}
