//! Thin Tauri command for the Word (.docx) export (TASK-020) — same
//! "build the real repositories/services from managed state, delegate,
//! no business logic here" pattern as every other `commands::*` module.
//! See `services::docx_export::DocxExportService`.

use crate::db::DbService;
use crate::errors::AppError;
use crate::media::MediaStorage;
use crate::repositories::capture::SqliteCaptureRepository;
use crate::repositories::process::SqliteProcessRepository;
use crate::repositories::process_version::SqliteProcessVersionRepository;
use crate::services::docx_export::DocxExportService;
use serde::Deserialize;
use std::path::PathBuf;
use tauri::State;

/// `target_path` is a path the user chose themselves through a native
/// Save As dialog on the frontend (`@tauri-apps/plugin-dialog`'s
/// `save()`) — the first, and so far only, place a frontend-supplied
/// filesystem path reaches native code in this app. It is still
/// validated here (must end in `.docx`, its parent directory must
/// exist), not trusted outright just because of where it came from —
/// see `DocxExportService::export`.
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
