//! `LatexExportService`: the LaTeX counterpart to `services::docx_export`
//! — same source data (a `ProcessVersion` plus its cited screenshot
//! Captures), same `export(version_id, target_path)` shape, same
//! validation discipline, deliberately kept as an independent sibling
//! service rather than merged behind a shared trait with
//! `DocxExportService` (see DECISIONS.md for why: the two formats'
//! output shapes — one file vs. a bundle — differ enough that a shared
//! abstraction would mostly be indirection, not shared logic).
//!
//! LaTeX has no native way to embed an image inline in the document
//! text the way `.docx`/`.tex` readers expect a single self-contained
//! file — `\includegraphics` only ever references a file on disk next
//! to the `.tex` source. So unlike the Word export's single `.docx`
//! file, this produces a `.zip` bundle: `document.tex` at its root, a
//! `images/<capture-id>.png` per embedded screenshot, and a short
//! `README.txt` explaining how to compile it. The user still picks one
//! file via a single native Save As dialog either way — the bundling is
//! an internal detail of what that one file contains.

use crate::errors::AppError;
use crate::media::MediaStorage;
use crate::models::capture::CaptureType;
use crate::models::process_version::ProcessVersion;
use crate::repositories::capture::CaptureRepository;
use crate::repositories::process::ProcessRepository;
use crate::repositories::process_version::ProcessVersionRepository;
use std::collections::HashMap;
use std::io::Write;
use std::path::Path;
use zip::write::SimpleFileOptions;
use zip::ZipWriter;

const README_TEXT: &str = "This is a LaTeX source bundle exported from GoLive.\r\n\
\r\n\
To produce a PDF:\r\n\
1. Unzip this archive, keeping document.tex and the images/ folder\r\n\
   together in the same directory.\r\n\
2. Compile document.tex with a LaTeX distribution (e.g. TeX Live,\r\n\
   MiKTeX, or an online editor such as Overleaf) — for example:\r\n\
   pdflatex document.tex\r\n";

pub struct LatexExportService {
    processes: Box<dyn ProcessRepository>,
    captures: Box<dyn CaptureRepository>,
    versions: Box<dyn ProcessVersionRepository>,
    media: MediaStorage,
}

impl LatexExportService {
    pub fn new(
        processes: Box<dyn ProcessRepository>,
        captures: Box<dyn CaptureRepository>,
        versions: Box<dyn ProcessVersionRepository>,
        media: MediaStorage,
    ) -> Self {
        Self { processes, captures, versions, media }
    }

    /// Generates a LaTeX source bundle (`document.tex` + `images/` +
    /// `README.txt`, zipped) for `version_id`'s content and writes it to
    /// `target_path`. Same error shape as `DocxExportService::export`:
    /// `NotFound` for a missing version/process, `Validation` for an
    /// unusable `target_path`, `Export` if writing the archive itself
    /// fails.
    pub fn export(&self, version_id: &str, target_path: &Path) -> Result<(), AppError> {
        let version = self.versions.get(version_id)?.ok_or(AppError::NotFound)?;
        let process = self.processes.get(&version.process_id)?.ok_or(AppError::NotFound)?;

        let has_zip_extension =
            target_path.extension().and_then(|ext| ext.to_str()).is_some_and(|ext| ext.eq_ignore_ascii_case("zip"));
        if !has_zip_extension {
            return Err(AppError::Validation("Choose a .zip file to export to.".to_string()));
        }
        if let Some(parent) = target_path.parent() {
            if !parent.as_os_str().is_empty() && !parent.exists() {
                return Err(AppError::Validation(
                    "The chosen export location no longer exists. Try Export again.".to_string(),
                ));
            }
        }

        let (document, screenshots) = self.build_document(&process.name, &process.description, &version);

        let file = std::fs::File::create(target_path).map_err(|err| {
            eprintln!("[golive] failed to create export file at {}: {err}", target_path.display());
            AppError::Export("Couldn't create the LaTeX bundle at that location. Try again.".to_string())
        })?;

        let mut zip = ZipWriter::new(file);
        let options = SimpleFileOptions::default().compression_method(zip::CompressionMethod::Deflated);
        let write_error = |err: zip::result::ZipError| {
            eprintln!("[golive] failed to write .zip export: {err}");
            AppError::Export("Couldn't write the LaTeX bundle. Try again.".to_string())
        };
        let io_error = |err: std::io::Error| {
            eprintln!("[golive] failed to write .zip export contents: {err}");
            AppError::Export("Couldn't write the LaTeX bundle. Try again.".to_string())
        };

        zip.start_file("document.tex", options).map_err(write_error)?;
        zip.write_all(document.as_bytes()).map_err(io_error)?;

        zip.start_file("README.txt", options).map_err(write_error)?;
        zip.write_all(README_TEXT.as_bytes()).map_err(io_error)?;

        for (capture_id, bytes) in &screenshots {
            zip.start_file(format!("images/{capture_id}.png"), options).map_err(write_error)?;
            zip.write_all(bytes).map_err(io_error)?;
        }

        zip.finish().map_err(write_error)?;

        Ok(())
    }

    /// Builds the `.tex` source text plus the screenshot bytes it
    /// references (by capture id, so the caller can write each one to
    /// `images/<id>.png` in the archive) — same "gather cited
    /// screenshots once, skip anything uncitable" approach as
    /// `DocxExportService::load_cited_screenshots`.
    fn build_document(
        &self,
        process_name: &str,
        process_description: &str,
        version: &ProcessVersion,
    ) -> (String, HashMap<String, Vec<u8>>) {
        let screenshots = self.load_cited_screenshots(version);

        let mut tex = String::new();
        tex.push_str(
            "\\documentclass[11pt]{article}\n\
             \\usepackage[margin=1in]{geometry}\n\
             \\usepackage[utf8]{inputenc}\n\
             \\usepackage[T1]{fontenc}\n\
             \\usepackage{graphicx}\n\
             \\usepackage{enumitem}\n\
             \\begin{document}\n\n",
        );

        tex.push_str("\\begin{center}\n");
        tex.push_str(&format!("{{\\Huge \\textbf{{{}}}}} \\\\[0.5em]\n", escape_latex(process_name)));
        tex.push_str("{\\Large Functional Specification}\n");
        tex.push_str("\\end{center}\n\n");

        if !process_description.trim().is_empty() {
            tex.push_str(&escape_latex_multiline(process_description));
            tex.push_str("\n\n");
        }

        tex.push_str("\\section*{Summary}\n");
        tex.push_str(&escape_latex_multiline(&version.content.summary));
        tex.push_str("\n\n");

        tex.push_str("\\section*{Process Steps}\n");
        tex.push_str("\\begin{enumerate}[leftmargin=*]\n");
        for step in &version.content.steps {
            tex.push_str(&format!("\\item \\textbf{{{}}}\n\n", escape_latex(&step.title)));
            if !step.description.trim().is_empty() {
                tex.push_str(&escape_latex_multiline(&step.description));
                tex.push('\n');
            }
            for capture_id in &step.capture_ids {
                if screenshots.contains_key(capture_id) {
                    tex.push_str(&format!(
                        "\n\\includegraphics[width=0.9\\textwidth]{{images/{capture_id}.png}}\n"
                    ));
                }
            }
            tex.push('\n');
        }
        tex.push_str("\\end{enumerate}\n\n");

        tex.push_str("\\end{document}\n");

        (tex, screenshots)
    }

    /// Same behavior as `DocxExportService::load_cited_screenshots` —
    /// duplicated rather than shared, since sharing it would mean the
    /// two otherwise-independent export services taking on a dependency
    /// on each other (or a new shared module) for four lines of logic.
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

/// Escapes the characters LaTeX treats specially so arbitrary
/// user/AI-authored text (which may contain `%`, `&`, `_`, `#`, `$`,
/// braces, backslashes, `~`, `^`) can never break compilation or be
/// misinterpreted as LaTeX markup. Applied to every piece of captured
/// text before it reaches the `.tex` source — nothing here is ever
/// trusted to already be LaTeX-safe.
fn escape_latex(text: &str) -> String {
    let mut escaped = String::with_capacity(text.len());
    for ch in text.chars() {
        match ch {
            '\\' => escaped.push_str("\\textbackslash{}"),
            '{' => escaped.push_str("\\{"),
            '}' => escaped.push_str("\\}"),
            '$' => escaped.push_str("\\$"),
            '&' => escaped.push_str("\\&"),
            '#' => escaped.push_str("\\#"),
            '%' => escaped.push_str("\\%"),
            '_' => escaped.push_str("\\_"),
            '~' => escaped.push_str("\\textasciitilde{}"),
            '^' => escaped.push_str("\\textasciicircum{}"),
            other => escaped.push(other),
        }
    }
    escaped
}

/// `escape_latex` plus turning embedded newlines into explicit LaTeX
/// line breaks within one paragraph (`\\`) — the same "one Paragraph,
/// explicit breaks between lines" treatment
/// `docx_export::wrapped_text_paragraph` gives multi-line text, applied
/// to LaTeX's own line-break syntax instead of a `docx-rs` `Run`.
fn escape_latex_multiline(text: &str) -> String {
    text.split('\n').map(escape_latex).collect::<Vec<_>>().join(" \\\\\n")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ai::{ProcessDraft, ProcessDraftStep};
    use crate::models::capture::Capture;
    use crate::models::process::{Process, ProcessStatus};
    use std::io::Read;

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
    ) -> LatexExportService {
        LatexExportService::new(
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
        let target = dir.path().join("out.zip");

        let result = service.export("does-not-exist", &target);

        assert!(matches!(result, Err(AppError::NotFound)));
    }

    #[test]
    fn export_rejects_a_target_path_without_a_zip_extension() {
        let dir = tempfile::tempdir().expect("temp dir");
        let steps = vec![ProcessDraftStep { title: "Step".to_string(), description: "Desc".to_string(), capture_ids: vec![] }];
        let service = build_service(Some(sample_process()), vec![], Some(sample_version(steps)), dir.path());
        let target = dir.path().join("out.tex");

        let result = service.export("v1", &target);

        assert!(matches!(result, Err(AppError::Validation(_))));
        assert!(!target.exists());
    }

    #[test]
    fn export_rejects_a_target_whose_parent_directory_does_not_exist() {
        let dir = tempfile::tempdir().expect("temp dir");
        let steps = vec![ProcessDraftStep { title: "Step".to_string(), description: "Desc".to_string(), capture_ids: vec![] }];
        let service = build_service(Some(sample_process()), vec![], Some(sample_version(steps)), dir.path());
        let target = dir.path().join("no-such-subdir").join("out.zip");

        let result = service.export("v1", &target);

        assert!(matches!(result, Err(AppError::Validation(_))));
    }

    #[test]
    fn export_writes_a_real_zip_containing_document_tex_and_a_readme() {
        let dir = tempfile::tempdir().expect("temp dir");
        let steps = vec![ProcessDraftStep {
            title: "Step one".to_string(),
            description: "Do the thing".to_string(),
            capture_ids: vec![],
        }];
        let service = build_service(Some(sample_process()), vec![], Some(sample_version(steps)), dir.path());
        let target = dir.path().join("out.zip");

        service.export("v1", &target).expect("export");

        assert!(target.exists());
        let file = std::fs::File::open(&target).expect("open exported file");
        let mut archive = zip::ZipArchive::new(file).expect("a real export must be a readable zip archive");

        let mut tex_contents = String::new();
        archive.by_name("document.tex").expect("document.tex must exist in the archive").read_to_string(&mut tex_contents).unwrap();
        assert!(tex_contents.contains("\\documentclass"));
        assert!(tex_contents.contains("Kickoff call"));
        assert!(tex_contents.contains("Step one"));
        assert!(tex_contents.contains("Do the thing"));

        assert!(archive.by_name("README.txt").is_ok(), "README.txt must exist in the archive");
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
        let target = dir.path().join("out.zip");

        service.export("v1", &target).expect("export");

        let file = std::fs::File::open(&target).expect("open exported file");
        let mut archive = zip::ZipArchive::new(file).expect("readable zip");
        assert!(
            archive.by_name("images/11111111-1111-1111-1111-111111111111.png").is_ok(),
            "the real screenshot's image must be embedded"
        );
        assert!(
            archive.by_name("images/22222222-2222-2222-2222-222222222222.png").is_err(),
            "a non-screenshot capture must not produce an image entry"
        );

        let mut tex_contents = String::new();
        archive.by_name("document.tex").unwrap().read_to_string(&mut tex_contents).unwrap();
        assert!(tex_contents.contains("images/11111111-1111-1111-1111-111111111111.png"));
    }

    #[test]
    fn escape_latex_escapes_every_special_character() {
        let input = "100% & #1 $cost_ {a} \\ ~tilde ^caret";
        let escaped = escape_latex(input);
        assert_eq!(
            escaped,
            "100\\% \\& \\#1 \\$cost\\_ \\{a\\} \\textbackslash{} \\textasciitilde{}tilde \\textasciicircum{}caret"
        );
    }

    #[test]
    fn escape_latex_multiline_joins_lines_with_a_latex_line_break() {
        let input = "First line\nSecond line with 50%";
        assert_eq!(escape_latex_multiline(input), "First line \\\\\nSecond line with 50\\%");
    }
}
