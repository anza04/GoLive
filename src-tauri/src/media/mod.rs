//! Application media storage: the filesystem boundary for captured media
//! (PNG screenshots today; recordings/audio later — see
//! docs/architecture.md, "Media storage boundary"). SQLite continues to
//! own Capture *metadata* only (see `repositories::capture`); this module
//! owns the actual bytes, kept under the same Tauri application-data
//! directory the database already lives in (`<data_dir>/captures`,
//! sibling to `<data_dir>/database`) — no second app-data mechanism is
//! invented.
//!
//! Deliberately has no dependency on Tauri types, same rationale as
//! `db::DbService`: takes a plain `&Path`, fully unit-testable with a
//! temp directory (see tests below).

use crate::errors::AppError;
use std::collections::HashSet;
use std::path::{Path, PathBuf};
use uuid::Uuid;

const CAPTURES_SUBDIR: &str = "captures";
const PNG_EXTENSION: &str = "png";
/// Container extension for Recording captures (TASK-013) — MP4, chosen
/// alongside `native::recording::WindowsRecordingEngine`'s encoder
/// (H.264 in an MP4 container via Windows' own Media Foundation) — see
/// DECISIONS.md.
const MP4_EXTENSION: &str = "mp4";
const MEDIA_EXTENSIONS: [&str; 2] = [PNG_EXTENSION, MP4_EXTENSION];

/// Cheap to clone (one `PathBuf`) — same "construct fresh per command,
/// backed by shared state" shape as `DbPool` (see `db::DbPool`), so
/// commands can hold a `tauri::State<MediaStorage>` and pass an owned
/// clone into each `CaptureService`.
#[derive(Clone)]
pub struct MediaStorage {
    captures_dir: PathBuf,
}

impl MediaStorage {
    /// Resolves `<data_dir>/captures` (creating it if needed) — a sibling
    /// of `<data_dir>/database` (see `db::DbService::init`). Idempotent:
    /// safe to call on every application startup, same as `DbService`;
    /// `create_dir_all` on an already-existing directory is a cheap no-op,
    /// not a full recreate.
    pub fn init(data_dir: &Path) -> Result<Self, AppError> {
        let captures_dir = data_dir.join(CAPTURES_SUBDIR);
        std::fs::create_dir_all(&captures_dir)?;
        Ok(Self { captures_dir })
    }

    /// Writes `bytes` as `<captures_dir>/<capture_id>.png`.
    pub fn save_capture(&self, capture_id: &str, bytes: &[u8]) -> Result<(), AppError> {
        let path = self.path_for(capture_id)?;
        std::fs::write(path, bytes)?;
        Ok(())
    }

    /// Reads the PNG bytes for `capture_id`. Returns `AppError::NotFound`
    /// if no media file exists for it — never a raw `io::Error`.
    pub fn read_capture(&self, capture_id: &str) -> Result<Vec<u8>, AppError> {
        let path = self.path_for(capture_id)?;
        std::fs::read(&path).map_err(|err| {
            if err.kind() == std::io::ErrorKind::NotFound {
                AppError::NotFound
            } else {
                AppError::from(err)
            }
        })
    }

    /// Deletes the PNG for `capture_id`. A missing file is treated as
    /// already-deleted success — never an error — so callers (e.g.
    /// `CaptureService::delete`) never have to special-case "there was
    /// nothing to clean up" (see docs/architecture.md, "Delete
    /// behavior").
    pub fn delete_capture(&self, capture_id: &str) -> Result<(), AppError> {
        let path = self.path_for(capture_id)?;
        match std::fs::remove_file(path) {
            Ok(()) => Ok(()),
            Err(err) if err.kind() == std::io::ErrorKind::NotFound => Ok(()),
            Err(err) => Err(AppError::from(err)),
        }
    }

    /// Resolves the on-disk path a Recording's video should be written
    /// to/read from: `<captures_dir>/<capture_id>.mp4`. Unlike the PNG
    /// methods above (which take a byte buffer already fully in memory),
    /// there is no `save_video`/`read_video` pair here — a recording is
    /// written incrementally, frame by frame, by
    /// `native::recording::WindowsRecordingEngine` while it's in
    /// progress, so the *path* is what the caller needs, not a
    /// save/read round trip. Same UUID validation as `path_for` — the
    /// entire path-traversal defense (see there).
    pub fn video_path(&self, capture_id: &str) -> Result<PathBuf, AppError> {
        self.path_for_extension(capture_id, MP4_EXTENSION)
    }

    /// Whether a video file exists for `capture_id` — used by
    /// `CaptureService::finalize_recording` to confirm the recording
    /// engine actually produced a file before creating the Capture
    /// metadata row (see docs/architecture.md).
    pub fn video_exists(&self, capture_id: &str) -> bool {
        self.video_path(capture_id).map(|path| path.exists()).unwrap_or(false)
    }

    /// Reads the full MP4 bytes for `capture_id` (TASK-014 playback) —
    /// the Recording counterpart to `read_capture`, same
    /// `AppError::NotFound`-for-a-missing-file behavior. A single
    /// in-memory byte-buffer transfer, the same approach `read_capture`
    /// already uses for PNGs — a deliberate, scoped choice (see
    /// DECISIONS.md) rather than a streaming/range-request protocol; it
    /// doesn't scale indefinitely to very long recordings, a limitation
    /// PROJECT_STATE.md already flags as unmeasured.
    pub fn read_video(&self, capture_id: &str) -> Result<Vec<u8>, AppError> {
        let path = self.video_path(capture_id)?;
        std::fs::read(&path).map_err(|err| {
            if err.kind() == std::io::ErrorKind::NotFound {
                AppError::NotFound
            } else {
                AppError::from(err)
            }
        })
    }

    /// Deletes the video for `capture_id` — the Recording counterpart to
    /// `delete_capture`, with the identical "missing file is already-
    /// deleted success" behavior. Called unconditionally alongside
    /// `delete_capture` by `CaptureService::delete` (a Capture is never
    /// both a screenshot and a recording, so at most one of the two
    /// calls ever finds a real file to remove).
    pub fn delete_video(&self, capture_id: &str) -> Result<(), AppError> {
        let path = self.video_path(capture_id)?;
        match std::fs::remove_file(path) {
            Ok(()) => Ok(()),
            Err(err) if err.kind() == std::io::ErrorKind::NotFound => Ok(()),
            Err(err) => Err(AppError::from(err)),
        }
    }

    /// Whether a media file exists for `capture_id`. Infallible by
    /// design — the same convention as `std::path::Path::exists`, which
    /// itself swallows I/O errors and reports `false`. An invalid id
    /// (see `path_for`) also just reports `false` rather than erroring,
    /// consistent with "does this exist?" never needing to fail.
    ///
    /// Part of `MediaStorage`'s required public surface (see
    /// docs/architecture.md, "Media storage boundary") even though no
    /// production caller needs it yet — `read_capture`'s own
    /// `AppError::NotFound` already answers "does this capture have
    /// media?" for `CaptureService::get_screenshot_media`'s one caller.
    /// Exercised directly by the tests below; kept for the same reason
    /// the rest of this API is deliberately a *complete*, symmetric
    /// save/read/delete/exists surface, ready for a future caller (e.g.
    /// a recording capture checking for media without reading it) rather
    /// than only whatever the current UI happens to need.
    #[allow(dead_code)]
    pub fn exists(&self, capture_id: &str) -> bool {
        self.path_for(capture_id).map(|path| path.exists()).unwrap_or(false)
    }

    /// Deletes every `.png`/`.mp4` file in the captures directory whose
    /// filename stem is not present in `known_capture_ids`. This is the
    /// media-storage cleanup boundary the Project/Process cascade
    /// limitation relies on (see docs/architecture.md, "Cascade media
    /// cleanup", and `services::capture::CaptureService::reconcile_media`,
    /// its only caller) — and, since TASK-013, also the safety net for an
    /// in-progress recording whose `stop_recording_capture` never ran
    /// (e.g. the app was closed mid-recording): the video file it left
    /// behind has no matching Capture row, so the next startup sweep
    /// removes it the same way it would any other orphan. Returns the
    /// number of orphaned files removed. An empty/missing captures
    /// directory is not an error — there is simply nothing to reconcile
    /// yet.
    pub fn reconcile(&self, known_capture_ids: &HashSet<String>) -> Result<usize, AppError> {
        let entries = match std::fs::read_dir(&self.captures_dir) {
            Ok(entries) => entries,
            Err(err) if err.kind() == std::io::ErrorKind::NotFound => return Ok(0),
            Err(err) => return Err(AppError::from(err)),
        };

        let mut removed = 0;
        for entry in entries {
            let path = entry?.path();
            let is_media_extension = path
                .extension()
                .and_then(|ext| ext.to_str())
                .is_some_and(|ext| MEDIA_EXTENSIONS.contains(&ext));
            if !is_media_extension {
                continue;
            }
            let Some(stem) = path.file_stem().and_then(|stem| stem.to_str()) else {
                continue;
            };
            if known_capture_ids.contains(stem) {
                continue;
            }
            match std::fs::remove_file(&path) {
                Ok(()) => removed += 1,
                Err(err) => eprintln!(
                    "[golive] failed to remove orphaned media file {}: {err}",
                    path.display()
                ),
            }
        }

        Ok(removed)
    }

    /// Resolves the safe on-disk path for `capture_id`'s PNG. Validates
    /// the id is a well-formed UUID before doing anything
    /// filesystem-related — the *only* path-safety check this module
    /// needs, since a valid UUID string can contain nothing but hex
    /// digits and hyphens, making path traversal (`../`, an absolute
    /// path, an arbitrary filename) structurally impossible regardless
    /// of what a caller passes in (see docs/architecture.md, "Safe media
    /// access").
    fn path_for(&self, capture_id: &str) -> Result<PathBuf, AppError> {
        self.path_for_extension(capture_id, PNG_EXTENSION)
    }

    /// Same validation/safety as `path_for`, generalized to any media
    /// extension this module knows about — `path_for` (PNG) and
    /// `video_path` (MP4, TASK-013) both delegate here.
    fn path_for_extension(&self, capture_id: &str, extension: &str) -> Result<PathBuf, AppError> {
        Uuid::parse_str(capture_id).map_err(|_| AppError::Validation("Invalid capture id.".to_string()))?;
        Ok(self.captures_dir.join(format!("{capture_id}.{extension}")))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn storage_in_temp_dir() -> (tempfile::TempDir, MediaStorage) {
        let dir = tempfile::tempdir().expect("temp dir");
        let storage = MediaStorage::init(dir.path()).expect("media storage init");
        (dir, storage)
    }

    fn sample_id() -> String {
        Uuid::new_v4().to_string()
    }

    #[test]
    fn init_creates_the_captures_directory() {
        let dir = tempfile::tempdir().expect("temp dir");
        MediaStorage::init(dir.path()).expect("init");
        assert!(dir.path().join("captures").is_dir());
    }

    #[test]
    fn init_is_idempotent() {
        let dir = tempfile::tempdir().expect("temp dir");
        MediaStorage::init(dir.path()).expect("first init");
        MediaStorage::init(dir.path()).expect("second init should not fail");
        assert!(dir.path().join("captures").is_dir());
    }

    #[test]
    fn save_then_read_round_trips_the_same_bytes() {
        let (_dir, storage) = storage_in_temp_dir();
        let id = sample_id();
        let bytes = vec![1, 2, 3, 4, 5];

        storage.save_capture(&id, &bytes).expect("save");
        let read_back = storage.read_capture(&id).expect("read");

        assert_eq!(read_back, bytes);
    }

    #[test]
    fn file_exists_after_save() {
        let (_dir, storage) = storage_in_temp_dir();
        let id = sample_id();

        assert!(!storage.exists(&id), "should not exist before save");
        storage.save_capture(&id, &[0u8]).expect("save");
        assert!(storage.exists(&id), "should exist after save");
    }

    #[test]
    fn generated_filename_is_based_on_capture_id() {
        let (dir, storage) = storage_in_temp_dir();
        let id = sample_id();

        storage.save_capture(&id, &[0u8]).expect("save");

        assert!(dir.path().join("captures").join(format!("{id}.png")).exists());
    }

    #[test]
    fn read_missing_capture_returns_not_found() {
        let (_dir, storage) = storage_in_temp_dir();
        let result = storage.read_capture(&sample_id());
        assert!(matches!(result, Err(AppError::NotFound)));
    }

    #[test]
    fn delete_removes_the_file() {
        let (_dir, storage) = storage_in_temp_dir();
        let id = sample_id();
        storage.save_capture(&id, &[0u8]).expect("save");

        storage.delete_capture(&id).expect("delete");

        assert!(!storage.exists(&id));
    }

    #[test]
    fn deleting_a_missing_file_is_handled_gracefully() {
        let (_dir, storage) = storage_in_temp_dir();
        // No save happened — deleting a capture id with no media must
        // not error (e.g. a Note/Recording capture, or a retry).
        storage.delete_capture(&sample_id()).expect("delete of missing file should be Ok");
    }

    #[test]
    fn paths_cannot_escape_the_media_directory() {
        let (dir, storage) = storage_in_temp_dir();

        for malicious_id in ["../../../etc/passwd", "..\\..\\windows\\system32", "/etc/passwd", "a/b"] {
            let result = storage.save_capture(malicious_id, &[0u8]);
            assert!(
                matches!(result, Err(AppError::Validation(_))),
                "expected {malicious_id:?} to be rejected as an invalid capture id"
            );
        }

        // Nothing was written outside (or even inside) the captures dir.
        let entries: Vec<_> = std::fs::read_dir(dir.path().join("captures"))
            .expect("read captures dir")
            .collect();
        assert!(entries.is_empty(), "no file should have been written from a malicious id");
    }

    #[test]
    fn reconcile_removes_only_orphaned_png_files() {
        let (dir, storage) = storage_in_temp_dir();
        let kept_id = sample_id();
        let orphan_id = sample_id();
        storage.save_capture(&kept_id, &[0u8]).expect("save kept");
        storage.save_capture(&orphan_id, &[0u8]).expect("save orphan");
        // A non-PNG/MP4 file must never be touched by reconciliation.
        std::fs::write(dir.path().join("captures").join("notes.txt"), b"hello").unwrap();

        let mut known = HashSet::new();
        known.insert(kept_id.clone());

        let removed = storage.reconcile(&known).expect("reconcile");

        assert_eq!(removed, 1);
        assert!(storage.exists(&kept_id), "known capture's media must survive reconciliation");
        assert!(!storage.exists(&orphan_id), "orphaned capture's media must be removed");
        assert!(dir.path().join("captures").join("notes.txt").exists(), "non-PNG files must be left alone");
    }

    #[test]
    fn video_path_is_the_capture_id_with_an_mp4_extension() {
        let (dir, storage) = storage_in_temp_dir();
        let id = sample_id();

        let path = storage.video_path(&id).expect("video_path");

        assert_eq!(path, dir.path().join("captures").join(format!("{id}.mp4")));
    }

    #[test]
    fn video_path_rejects_a_malicious_id() {
        let (_dir, storage) = storage_in_temp_dir();
        let result = storage.video_path("../../../etc/passwd");
        assert!(matches!(result, Err(AppError::Validation(_))));
    }

    #[test]
    fn video_exists_reflects_whether_the_file_is_on_disk() {
        let (_dir, storage) = storage_in_temp_dir();
        let id = sample_id();

        assert!(!storage.video_exists(&id), "should not exist before it's written");
        std::fs::write(storage.video_path(&id).unwrap(), b"not a real mp4, just bytes").unwrap();
        assert!(storage.video_exists(&id), "should exist once the file is on disk");
    }

    #[test]
    fn read_video_round_trips_the_same_bytes() {
        let (_dir, storage) = storage_in_temp_dir();
        let id = sample_id();
        let bytes = vec![1, 2, 3, 4, 5];
        std::fs::write(storage.video_path(&id).unwrap(), &bytes).unwrap();

        let read_back = storage.read_video(&id).expect("read");
        assert_eq!(read_back, bytes);
    }

    #[test]
    fn read_missing_video_returns_not_found() {
        let (_dir, storage) = storage_in_temp_dir();
        let result = storage.read_video(&sample_id());
        assert!(matches!(result, Err(AppError::NotFound)));
    }

    #[test]
    fn delete_video_removes_the_file_and_is_a_graceful_no_op_when_missing() {
        let (_dir, storage) = storage_in_temp_dir();
        let id = sample_id();
        std::fs::write(storage.video_path(&id).unwrap(), b"bytes").unwrap();

        storage.delete_video(&id).expect("delete");
        assert!(!storage.video_exists(&id));

        // Deleting again (nothing left to delete) must still succeed.
        storage.delete_video(&id).expect("delete of missing video should be Ok");
    }

    #[test]
    fn reconcile_also_removes_orphaned_mp4_files_but_keeps_known_ones() {
        let (_dir, storage) = storage_in_temp_dir();
        let kept_id = sample_id();
        let orphan_id = sample_id();
        std::fs::write(storage.video_path(&kept_id).unwrap(), b"kept").unwrap();
        std::fs::write(storage.video_path(&orphan_id).unwrap(), b"orphaned").unwrap();

        let mut known = HashSet::new();
        known.insert(kept_id.clone());

        let removed = storage.reconcile(&known).expect("reconcile");

        assert_eq!(removed, 1);
        assert!(storage.video_exists(&kept_id), "known recording's media must survive reconciliation");
        assert!(!storage.video_exists(&orphan_id), "orphaned recording's media must be removed");
    }

    #[test]
    fn reconcile_on_a_missing_directory_is_a_safe_no_op() {
        let dir = tempfile::tempdir().expect("temp dir");
        let storage = MediaStorage::init(dir.path()).expect("init");
        std::fs::remove_dir_all(dir.path().join("captures")).unwrap();

        let removed = storage.reconcile(&HashSet::new()).expect("reconcile should not error");
        assert_eq!(removed, 0);
    }
}
