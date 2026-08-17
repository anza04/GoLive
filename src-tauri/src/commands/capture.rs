//! Thin Tauri-facing commands for the Capture domain. Each one just
//! builds a `CaptureService` from the managed `DbService` and delegates —
//! no business logic, no SQL (same pattern as `commands::process`).

use crate::db::DbService;
use crate::errors::AppError;
use crate::models::capture::Capture;
use crate::repositories::capture::SqliteCaptureRepository;
use crate::repositories::process::SqliteProcessRepository;
use crate::services::capture::CaptureService;
use serde::Deserialize;
use tauri::State;

#[derive(Deserialize)]
pub struct CreateCaptureInput {
    pub process_id: String,
    pub capture_type: String,
    pub title: String,
    #[serde(default)]
    pub description: Option<String>,
}

/// Wraps the single `process_id` scalar in an explicit input struct —
/// same rationale as `commands::process::ListProcessesInput` (see there).
#[derive(Deserialize)]
pub struct ListCapturesInput {
    pub process_id: String,
}

/// Explicit update input — deliberately not the `Capture` model itself,
/// so an update request has no field for `process_id`/`created_at`/
/// `updated_at` to occupy. A capture's process cannot be changed through
/// update.
#[derive(Deserialize)]
pub struct UpdateCaptureInput {
    pub id: String,
    pub capture_type: String,
    pub title: String,
    #[serde(default)]
    pub description: Option<String>,
}

fn service(db: &State<DbService>) -> CaptureService {
    CaptureService::new(
        Box::new(SqliteCaptureRepository::new(db.pool())),
        Box::new(SqliteProcessRepository::new(db.pool())),
    )
}

#[tauri::command]
pub fn create_capture(input: CreateCaptureInput, db: State<DbService>) -> Result<Capture, AppError> {
    service(&db).create(&input.process_id, &input.capture_type, &input.title, input.description.as_deref())
}

#[tauri::command]
pub fn list_captures(input: ListCapturesInput, db: State<DbService>) -> Result<Vec<Capture>, AppError> {
    service(&db).list_by_process(&input.process_id)
}

#[tauri::command]
pub fn get_capture(id: String, db: State<DbService>) -> Result<Capture, AppError> {
    service(&db).get(&id)
}

#[tauri::command]
pub fn update_capture(input: UpdateCaptureInput, db: State<DbService>) -> Result<Capture, AppError> {
    service(&db).update(&input.id, &input.capture_type, &input.title, input.description.as_deref())
}

#[tauri::command]
pub fn delete_capture(id: String, db: State<DbService>) -> Result<(), AppError> {
    service(&db).delete(&id)
}
