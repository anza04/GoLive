//! Thin Tauri commands for document export (TASK-020 Word, follow-up
//! LaTeX) — same "build the real repositories/services from managed
//! state, delegate, no business logic here" pattern as every other
//! `commands::*` module. See `services::docx_export::DocxExportService`
//! and `services::latex_export::LatexExportService`.

use crate::db::DbService;
use crate::errors::AppError;
use crate::media::MediaStorage;
use crate::repositories::capture::SqliteCaptureRepository;
use crate::repositories::process::SqliteProcessRepository;
use crate::repositories::process_version::SqliteProcessVersionRepository;
use crate::services::docx_export::DocxExportService;
use crate::services::latex_export::LatexExportService;
use serde::Deserialize;
use std::path::PathBuf;
use tauri::State;

/// `target_path` is a path the user chose themselves through a native
/// Save As dialog on the frontend (`@tauri-apps/plugin-dialog`'s
/// `save()`) — the first place a frontend-supplied filesystem path
/// reaches native code in this app. It is still validated here (must
/// end in the format's own extension, its parent directory must exist),
/// not trusted outright just because of where it came from — see
/// `DocxExportService::export`/`LatexExportService::export`.
#[derive(Deserialize)]
pub struct ExportProcessVersionInput {
    pub version_id: String,
    pub target_path: String,
}

/// Generates a `.docx` functional specification for `input.version_id`'s
/// content and writes it to `input.target_path`.
#[tauri::command]
pub fn export_process_version_to_docx(
    input: ExportProcessVersionInput,
    db: State<DbService>,
    media: State<MediaStorage>,
) -> Result<(), AppError> {
    let service = DocxExportService::new(
        Box::new(SqliteProcessRepository::new(db.pool())),
        Box::new(SqliteCaptureRepository::new(db.pool())),
        Box::new(SqliteProcessVersionRepository::new(db.pool())),
        media.inner().clone(),
    );
    service.export(&input.version_id, &PathBuf::from(input.target_path))
}

/// Generates a LaTeX source bundle (`.zip` — `document.tex`, embedded
/// screenshots under `images/`, a `README.txt`) for
/// `input.version_id`'s content and writes it to `input.target_path`.
#[tauri::command]
pub fn export_process_version_to_latex(
    input: ExportProcessVersionInput,
    db: State<DbService>,
    media: State<MediaStorage>,
) -> Result<(), AppError> {
    let service = LatexExportService::new(
        Box::new(SqliteProcessRepository::new(db.pool())),
        Box::new(SqliteCaptureRepository::new(db.pool())),
        Box::new(SqliteProcessVersionRepository::new(db.pool())),
        media.inner().clone(),
    );
    service.export(&input.version_id, &PathBuf::from(input.target_path))
}
