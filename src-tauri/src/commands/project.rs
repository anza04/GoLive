//! Thin Tauri-facing commands for the Project domain. Each one just
//! builds a `ProjectService` from the managed `DbService` and delegates —
//! no business logic, no SQL (same pattern as `commands::storage`).

use crate::db::DbService;
use crate::errors::AppError;
use crate::models::project::Project;
use crate::repositories::project::SqliteProjectRepository;
use crate::services::project::ProjectService;
use serde::Deserialize;
use tauri::State;

#[derive(Deserialize)]
pub struct CreateProjectInput {
    pub name: String,
    #[serde(default)]
    pub description: Option<String>,
}

/// Explicit update input — deliberately not the `Project` model itself,
/// so an update request has no field for `created_at`/`updated_at` to
/// accidentally (or maliciously) occupy. Only `id`/`name`/`description`
/// exist here; the backend supplies everything else.
#[derive(Deserialize)]
pub struct UpdateProjectInput {
    pub id: String,
    pub name: String,
    #[serde(default)]
    pub description: Option<String>,
}

fn service(db: &State<DbService>) -> ProjectService {
    ProjectService::new(Box::new(SqliteProjectRepository::new(db.pool())))
}

#[tauri::command]
pub fn create_project(input: CreateProjectInput, db: State<DbService>) -> Result<Project, AppError> {
    service(&db).create(&input.name, input.description.as_deref())
}

#[tauri::command]
pub fn update_project(input: UpdateProjectInput, db: State<DbService>) -> Result<Project, AppError> {
    service(&db).update(&input.id, &input.name, input.description.as_deref())
}

#[tauri::command]
pub fn list_projects(db: State<DbService>) -> Result<Vec<Project>, AppError> {
    service(&db).list()
}

#[tauri::command]
pub fn get_project(id: String, db: State<DbService>) -> Result<Project, AppError> {
    service(&db).get(&id)
}

#[tauri::command]
pub fn delete_project(id: String, db: State<DbService>) -> Result<(), AppError> {
    service(&db).delete(&id)
}
