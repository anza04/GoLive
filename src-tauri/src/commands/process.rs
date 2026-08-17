//! Thin Tauri-facing commands for the Process domain. Each one just
//! builds a `ProcessService` from the managed `DbService` and delegates —
//! no business logic, no SQL (same pattern as `commands::project`).

use crate::db::DbService;
use crate::errors::AppError;
use crate::models::process::Process;
use crate::repositories::process::SqliteProcessRepository;
use crate::repositories::project::SqliteProjectRepository;
use crate::services::process::ProcessService;
use serde::Deserialize;
use tauri::State;

#[derive(Deserialize)]
pub struct CreateProcessInput {
    pub project_id: String,
    pub name: String,
    #[serde(default)]
    pub description: Option<String>,
}

/// Wraps the single `project_id` scalar in an explicit input struct
/// (rather than a bare `project_id: String` command parameter) so the
/// frontend passes it as a literal, unambiguous struct field
/// (`{ input: { project_id } }`) instead of depending on Tauri's default
/// camelCase argument-name conversion for a multi-word parameter name —
/// avoids a class of mismatch that's easy to get wrong and hard to catch
/// in this project's mocked-IPC frontend tests (see DECISIONS.md).
#[derive(Deserialize)]
pub struct ListProcessesInput {
    pub project_id: String,
}

/// Explicit update input — deliberately not the `Process` model itself,
/// so an update request has no field for `project_id`/`created_at`/
/// `updated_at` to occupy. A process's project cannot be changed through
/// update (see docs/architecture.md, "Project ownership").
#[derive(Deserialize)]
pub struct UpdateProcessInput {
    pub id: String,
    pub name: String,
    #[serde(default)]
    pub description: Option<String>,
    pub status: String,
}

fn service(db: &State<DbService>) -> ProcessService {
    ProcessService::new(
        Box::new(SqliteProcessRepository::new(db.pool())),
        Box::new(SqliteProjectRepository::new(db.pool())),
    )
}

#[tauri::command]
pub fn create_process(input: CreateProcessInput, db: State<DbService>) -> Result<Process, AppError> {
    service(&db).create(&input.project_id, &input.name, input.description.as_deref())
}

#[tauri::command]
pub fn list_processes(input: ListProcessesInput, db: State<DbService>) -> Result<Vec<Process>, AppError> {
    service(&db).list_by_project(&input.project_id)
}

#[tauri::command]
pub fn get_process(id: String, db: State<DbService>) -> Result<Process, AppError> {
    service(&db).get(&id)
}

#[tauri::command]
pub fn update_process(input: UpdateProcessInput, db: State<DbService>) -> Result<Process, AppError> {
    service(&db).update(&input.id, &input.name, input.description.as_deref(), &input.status)
}

#[tauri::command]
pub fn delete_process(id: String, db: State<DbService>) -> Result<(), AppError> {
    service(&db).delete(&id)
}
