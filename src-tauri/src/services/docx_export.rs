//! `DocxExportService` (TASK-020): builds a Word (.docx) functional
//! specification from a Process's structured content — a `ProcessVersion`
//! (TASK-018, editable since TASK-019) — with screenshots embedded next
//! to the step that cites them (`ProcessDraftStep::capture_ids`, which
//! `ai::mod` already noted this task would need). The final M3 step:
//! turns a captured, AI-structured, user-edited process into the
//! product's actual deliverable. See docs/architecture.md, "Word export".
//!
//! Sits at the same layer as `ProcessDraftService` — reads through the
//! existing repositories/`MediaStorage`, writes nothing back to them.
//! The one genuinely new thing this task does: writes a file to a path
//! the *user* chose (a native Save As dialog, driven by the frontend —
//! see `commands::export`), the first time anything in GoLive does that.
//! `export` still validates `target_path` itself rather than trusting it
//! outright, even though it only ever originates from that native
//! dialog, not arbitrary frontend input.

use crate::errors::AppError;
use crate::media::MediaStorage;
use crate::models::capture::CaptureType;
use crate::models::process_version::ProcessVersion;
use crate::repositories::capture::CaptureRepository;
use crate::repositories::process::ProcessRepository;
use crate::repositories::process_version::ProcessVersionRepository;
use docx_rs::{AlignmentType, BreakType, Docx, Paragraph, Pic, Run};
use std::collections::HashMap;
use std::path::Path;

/// Word's usable page width at the default Letter page size and 1-inch
/// margins (8.5in page - 2 * 1in margin = 6.5in), in EMUs (914400 per
/// inch), trimmed slightly further for a small visual gutter. A
/// screenshot wider than this is scaled down (never up) so it never
/// overflows the page — `Pic::new` alone sizes an image 1:1 to its pixel
/// dimensions, which for a real screen-resolution screenshot would be
/// many times too wide.
const MAX_IMAGE_WIDTH_EMU: u32 = 6 * 914_400;

pub struct DocxExportService {
    processes: Box<dyn ProcessRepository>,
    captures: Box<dyn CaptureRepository>,
    versions: Box<dyn ProcessVersionRepository>,
    media: MediaStorage,
}

impl DocxExportService {
    pub fn new(
        processes: Box<dyn ProcessRepository>,
        captures: Box<dyn CaptureRepository>,
        versions: Box<dyn ProcessVersionRepository>,
        media: MediaStorage,
    ) -> Self {
        Self { processes, captures, versions, media }
    }

    /// Generates a `.docx` functional specification for `version_id`'s
    /// content and writes it to `target_path`. Returns `AppError::NotFound`
    /// if the version (or its parent Process) doesn't exist,
    /// `AppError::Validation` if `target_path` is obviously unusable, and
    /// `AppError::Export` if building or writing the document itself
    /// fails.
    pub fn export(&self, version_id: &str, target_path: &Path) -> Result<(), AppError> {
        let version = self.versions.get(version_id)?.ok_or(AppError::NotFound)?;
        let process = self.processes.get(&version.process_id)?.ok_or(AppError::NotFound)?;

        let has_docx_extension = target_path
            .extension()
            .and_then(|ext| ext.to_str())
            .is_some_and(|ext| ext.eq_ignore_ascii_case("docx"));
        if !has_docx_extension {
            return Err(AppError::Validation("Choose a .docx file to export to.".to_string()));
        }
        if let Some(parent) = target_path.parent() {
            if !parent.as_os_str().is_empty() && !parent.exists() {
                return Err(AppError::Validation(
                    "The chosen export location no longer exists. Try Export again.".to_string(),
                ));
            }
        }

        let docx = self.build_document(&process.name, &process.description, &version);

        let file = std::fs::File::create(target_path).map_err(|err| {
            eprintln!("[golive] failed to create export file at {}: {err}", target_path.display());
            AppError::Export("Couldn't create the Word document at that location. Try again.".to_string())
        })?;
        docx.pack(file).map_err(|err| {
            eprintln!("[golive] failed to write .docx export: {err}");
            AppError::Export("Couldn't write the Word document. Try again.".to_string())
        })?;

        Ok(())
    }

    fn build_document(&self, process_name: &str, process_description: &str, version: &ProcessVersion) -> Docx {
        let screenshots = self.load_cited_screenshots(version);

        let mut docx = Docx::new();

        docx = docx.add_paragraph(
            Paragraph::new()
                .add_run(Run::new().add_text(process_name).bold().size(56))
                .align(AlignmentType::Center),
        );
        docx = docx.add_paragraph(
            Paragraph::new()
                .add_run(Run::new().add_text("Functional Specification").size(28))
                .align(AlignmentType::Center),
        );

        if !process_description.trim().is_empty() {
            docx = docx.add_paragraph(wrapped_text_paragraph(process_description, 22));
        }

        docx = docx.add_paragraph(Paragraph::new().add_run(Run::new().add_text("Summary").bold().size(32)));
        docx = docx.add_paragraph(wrapped_text_paragraph(&version.content.summary, 22));

        docx = docx.add_paragraph(Paragraph::new().add_run(Run::new().add_text("Process Steps").bold().size(32)));

        for (index, step) in version.content.steps.iter().enumerate() {
            docx = docx.add_paragraph(
                Paragraph::new()
                    .add_run(Run::new().add_text(format!("{}. {}", index + 1, step.title)).bold().size(26)),
            );
            if !step.description.trim().is_empty() {
                docx = docx.add_paragraph(wrapped_text_paragraph(&step.description, 22));
            }
            for capture_id in &step.capture_ids {
                if let Some(bytes) = screenshots.get(capture_id) {
                    docx = docx.add_paragraph(Paragraph::new().add_run(Run::new().add_image(scaled_pic(bytes))));
                }
            }
        }

        docx
    }

    /// Reads screenshot bytes for every capture id any step cites, once
    /// each — a capture cited by more than one step would otherwise be
    /// read from disk repeatedly. Only screenshot Captures have image
    /// bytes at all; a Recording/Note id cited by a step (or a
    /// screenshot whose PNG has since gone missing) is silently skipped,
    /// same "describe what's actually there" tolerance
    /// `services::process_draft::ProcessDraftService::generate` already
    /// applies to missing media — a missing image must not fail the
    /// whole export.
    fn load_cited_screenshots(&self, version: &ProcessVersion) -> HashMap<String, Vec<u8>> {
        let mut screenshots = HashMap::new();
        for step in &version.content.steps {
            for capture_id in &step.capture_ids {
                if screenshots.contains_key(capture_id) {
                    continue;
                }
                let Ok(Some(capture)) = self.captures.get(capture_id) else { continue };
                if capture.capture_type != CaptureType::Screenshot {
                    continue;
                }
                if let Ok(bytes) = self.media.read_capture(capture_id) {
                    screenshots.insert(capture_id.clone(), bytes);
                }
            }
        }
        screenshots
    }
}

/// Splits `text` on newlines into one `Run` per line joined by explicit
/// text-wrapping breaks within a single `Paragraph` — a Word paragraph
/// doesn't otherwise honor embedded `\n` characters (a multi-line
/// summary/step description would otherwise render as one run-on line).
fn wrapped_text_paragraph(text: &str, size: usize) -> Paragraph {
    let lines: Vec<&str> = text.split('\n').collect();
    let mut paragraph = Paragraph::new();
    for (index, line) in lines.iter().enumerate() {
        let mut run = Run::new().add_text(*line).size(size);
        if index + 1 < lines.len() {
            run = run.add_break(BreakType::TextWrapping);
        }
        paragraph = paragraph.add_run(run);
    }
    paragraph
}

/// A `Pic` for `bytes` (an already-confirmed screenshot PNG), scaled
/// down — never up — to `MAX_IMAGE_WIDTH_EMU`, preserving aspect ratio.
fn scaled_pic(bytes: &[u8]) -> Pic {
    let pic = Pic::new(bytes);
    let (width, height) = pic.size;
    if width == 0 || width <= MAX_IMAGE_WIDTH_EMU {
        return pic;
    }
    let scale = MAX_IMAGE_WIDTH_EMU as f64 / width as f64;
    let scaled_height = (height as f64 * scale).round().max(1.0) as u32;
    pic.size(MAX_IMAGE_WIDTH_EMU, scaled_height)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ai::{ProcessDraft, ProcessDraftStep};
    use crate::models::capture::Capture;
    use crate::models::process::{Process, ProcessStatus};

    struct FakeProcessRepository {
        process: Option<Process>,
    }
    impl ProcessRepository for FakeProcessRepository {
        fn create(&self, _process: &Process) -> Result<(), AppError> {
            unreachable!()
        }
        fn list_by_project(&self, _project_id: &str) -> Result<Vec<Process>, AppError> {
            unreachable!()
        }
        fn get(&self, _id: &str) -> Result<Option<Process>, AppError> {
            Ok(self.process.clone())
        }
        fn update(&self, _process: &Process) -> Result<bool, AppError> {
            unreachable!()
        }
        fn delete(&self, _id: &str) -> Result<bool, AppError> {
            unreachable!()
        }
    }

    struct FakeCaptureRepository {
        captures: Vec<Capture>,
    }
    impl CaptureRepository for FakeCaptureRepository {
        fn create(&self, _capture: &Capture) -> Result<(), AppError> {
            unreachable!()
        }
        fn list_by_process(&self, _process_id: &str) -> Result<Vec<Capture>, AppError> {
            unreachable!()
        }
        fn get(&self, id: &str) -> Result<Option<Capture>, AppError> {
            Ok(self.captures.iter().find(|c| c.id == id).cloned())
        }
        fn update(&self, _capture: &Capture) -> Result<bool, AppError> {
            unreachable!()
        }
        fn delete(&self, _id: &str) -> Result<bool, AppError> {
            unreachable!()
        }
        fn list_all_ids(&self) -> Result<Vec<String>, AppError> {
            unreachable!()
        }
    }

    struct FakeProcessVersionRepository {
        version: Option<ProcessVersion>,
    }
    impl ProcessVersionRepository for FakeProcessVersionRepository {
        fn create(&self, _version: &ProcessVersion) -> Result<(), AppError> {
            unreachable!()
        }
        fn list_by_process(&self, _process_id: &str) -> Result<Vec<ProcessVersion>, AppError> {
            unreachable!()
        }
        fn get_latest_by_process(&self, _process_id: &str) -> Result<Option<ProcessVersion>, AppError> {
            unreachable!()
        }
        fn get(&self, _id: &str) -> Result<Option<ProcessVersion>, AppError> {
            Ok(self.version.clone())
        }
        fn update_content(&self, _id: &str, _content: &ProcessDraft, _updated_at: i64) -> Result<bool, AppError> {
            unreachable!()
        }
    }

    fn sample_process() -> Process {
        Process {
            id: "proc-1".to_string(),
            project_id: "proj-1".to_string(),
            name: "Kickoff call".to_string(),
            description: "First client call".to_string(),
            status: ProcessStatus::InProgress,
            created_at: 0,
            updated_at: 0,
        }
    }

    fn sample_capture(id: &str, capture_type: CaptureType) -> Capture {
        Capture {
            id: id.to_string(),
            process_id: "proc-1".to_string(),
            capture_type,
            title: format!("Capture {id}"),
            description: String::new(),
            created_at: 1,
            updated_at: 1,
        }
    }

    fn sample_version(steps: Vec<ProcessDraftStep>) -> ProcessVersion {
        ProcessVersion {
            id: "v1".to_string(),
            process_id: "proc-1".to_string(),
            content: ProcessDraft { summary: "A summary".to_string(), steps },
            created_at: 1000,
            updated_at: 1000,
        }
    }

    fn one_pixel_png() -> Vec<u8> {
        // A minimal valid 1x1 PNG, small enough to embed directly in a
        // test — real Pic sizing (`scaled_pic`) only needs `Pic::new` to
        // successfully read a width/height from it.
        use base64::Engine;
        base64::engine::general_purpose::STANDARD
            .decode("iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAQAAAC1HAwCAAAAC0lEQVR42mNk+A8AAQUBAScY42YAAAAASUVORK5CYII=")
            .expect("valid base64")
    }

    fn build_service(
        process: Option<Process>,
        captures: Vec<Capture>,
        version: Option<ProcessVersion>,
        media_dir: &std::path::Path,
    ) -> DocxExportService {
        DocxExportService::new(
            Box::new(FakeProcessRepository { process }),
            Box::new(FakeCaptureRepository { captures }),
            Box::new(FakeProcessVersionRepository { version }),
            MediaStorage::init(media_dir).expect("media storage init"),
        )
    }

    #[test]
    fn export_rejects_a_missing_version() {
        let dir = tempfile::tempdir().expect("temp dir");
        let service = build_service(Some(sample_process()), vec![], None, dir.path());
        let target = dir.path().join("out.docx");

        let result = service.export("does-not-exist", &target);

        assert!(matches!(result, Err(AppError::NotFound)));
    }

    #[test]
    fn export_rejects_a_target_path_without_a_docx_extension() {
        let dir = tempfile::tempdir().expect("temp dir");
        let steps = vec![ProcessDraftStep { title: "Step".to_string(), description: "Desc".to_string(), capture_ids: vec![] }];
        let service = build_service(Some(sample_process()), vec![], Some(sample_version(steps)), dir.path());
        let target = dir.path().join("out.txt");

        let result = service.export("v1", &target);

        assert!(matches!(result, Err(AppError::Validation(_))));
        assert!(!target.exists());
    }

    #[test]
    fn export_rejects_a_target_whose_parent_directory_does_not_exist() {
        let dir = tempfile::tempdir().expect("temp dir");
        let steps = vec![ProcessDraftStep { title: "Step".to_string(), description: "Desc".to_string(), capture_ids: vec![] }];
        let service = build_service(Some(sample_process()), vec![], Some(sample_version(steps)), dir.path());
        let target = dir.path().join("no-such-subdir").join("out.docx");

        let result = service.export("v1", &target);

        assert!(matches!(result, Err(AppError::Validation(_))));
    }

    #[test]
    fn export_writes_a_real_docx_file_for_a_version_with_no_captures() {
        let dir = tempfile::tempdir().expect("temp dir");
        let steps = vec![ProcessDraftStep {
            title: "Step one".to_string(),
            description: "Do the thing".to_string(),
            capture_ids: vec![],
        }];
        let service = build_service(Some(sample_process()), vec![], Some(sample_version(steps)), dir.path());
        let target = dir.path().join("out.docx");

        service.export("v1", &target).expect("export");

        assert!(target.exists());
        // A .docx is a ZIP container — a real one starts with the ZIP
        // local-file-header magic bytes, not just "some non-empty file".
        let bytes = std::fs::read(&target).expect("read exported file");
        assert_eq!(&bytes[0..2], b"PK", "exported file should be a real ZIP/.docx container");
    }

    #[test]
    fn export_embeds_a_cited_screenshot_and_skips_a_missing_or_non_screenshot_capture() {
        let dir = tempfile::tempdir().expect("temp dir");
        let media = MediaStorage::init(dir.path()).expect("media init");
        media.save_capture("11111111-1111-1111-1111-111111111111", &one_pixel_png()).expect("save screenshot");

        let captures = vec![
            sample_capture("11111111-1111-1111-1111-111111111111", CaptureType::Screenshot),
            sample_capture("22222222-2222-2222-2222-222222222222", CaptureType::Note),
        ];
        let steps = vec![ProcessDraftStep {
            title: "Step one".to_string(),
            description: "Do the thing".to_string(),
            capture_ids: vec![
                "11111111-1111-1111-1111-111111111111".to_string(),
                "22222222-2222-2222-2222-222222222222".to_string(),
                "no-such-capture".to_string(),
            ],
        }];
        let service = build_service(Some(sample_process()), captures, Some(sample_version(steps)), dir.path());
        let target = dir.path().join("out.docx");

        // Must not fail just because two of the three cited ids have no
        // embeddable image — the real screenshot is still embedded.
        service.export("v1", &target).expect("export");

        assert!(target.exists());
    }

    #[test]
    fn scaled_pic_downscales_a_too_wide_image_but_never_upscales_a_narrow_one() {
        let pic = scaled_pic(&one_pixel_png());
        // A 1x1 PNG is far narrower than MAX_IMAGE_WIDTH_EMU — its size
        // must be left exactly as `Pic::new` computed it, not stretched.
        assert_eq!(pic.size.0, docx_rs::Pic::new(&one_pixel_png()).size.0);
    }
}
