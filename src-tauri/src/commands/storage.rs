//! The proof-of-persistence command: exercises the full
//! React -> service -> Tauri command -> repository -> SQLite -> repository
//! -> Tauri -> React round trip without modeling any real domain data.

use crate::db::DbService;
use crate::errors::AppError;
use crate::repositories::storage_status::{SqliteStorageStatusRepository, StorageStatusRepository};
use serde::Serialize;
use tauri::State;

#[derive(Serialize)]
pub struct LocalStorageStatus {
    pub ready: bool,
    /// Unix timestamp (seconds) of when local storage was first
    /// initialized on this machine. Not shown as primary UI text — kept
    /// available for a debug tooltip, matching how backend connectivity
    /// detail is handled (see docs/architecture.md, "Error handling").
    pub initialized_at: String,
}

#[tauri::command]
pub fn get_local_storage_status(db: State<DbService>) -> Result<LocalStorageStatus, AppError> {
    let repo = SqliteStorageStatusRepository::new(db.pool());
    let initialized_at = repo.ensure_marker()?;
    Ok(LocalStorageStatus {
        ready: true,
        initialized_at,
    })
}
