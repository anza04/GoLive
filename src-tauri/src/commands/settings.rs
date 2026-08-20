//! Thin Tauri-facing commands for Settings (TASK-016: the OpenAI API
//! key). Each one just builds a `SettingsService` and delegates — no
//! business logic, no direct credential-store or network calls (same
//! pattern as every other `commands::*` module). `SettingsService`
//! itself holds no shared state worth managing via `tauri::State` (see
//! `credentials::WindowsCredentialStore` — a zero-field unit struct), so
//! unlike `commands::capture`/`commands::process` there is no `DbService`/
//! `MediaStorage` to pull out of Tauri's managed state here.

use crate::credentials::WindowsCredentialStore;
use crate::errors::AppError;
use crate::openai;
use crate::services::settings::SettingsService;
use serde::Deserialize;

/// Same "wrap the single scalar in an explicit input struct" convention
/// `commands::process::ListProcessesInput` documents (see there) — avoids
/// depending on Tauri's default camelCase argument-name conversion for a
/// single named parameter.
#[derive(Deserialize)]
pub struct SaveApiKeyInput {
    pub api_key: String,
}

fn service() -> SettingsService {
    SettingsService::new(Box::new(WindowsCredentialStore))
}

#[tauri::command]
pub fn save_api_key(input: SaveApiKeyInput) -> Result<(), AppError> {
    service().save_api_key(&input.api_key)
}

#[tauri::command]
pub fn has_api_key() -> Result<bool, AppError> {
    service().has_api_key()
}

#[tauri::command]
pub fn clear_api_key() -> Result<(), AppError> {
    service().clear_api_key()
}

/// Tests the *currently saved* key against OpenAI (see roadmap.md
/// TASK-016) — not whatever might be sitting unsaved in the Settings
/// form. `openai::test_api_key` is a blocking network call; Tauri runs
/// synchronous commands like this one on its own worker thread pool, not
/// the event loop, so this does not freeze either window's UI while it
/// runs (same reasoning already applies to every blocking SQLite call
/// elsewhere in this codebase).
#[tauri::command]
pub fn test_api_key_connection() -> Result<(), AppError> {
    service().test_connection(openai::test_api_key)
}
