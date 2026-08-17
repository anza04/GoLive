//! Application-level error type.
//!
//! Established here (TASK-004) as documented in docs/architecture.md
//! ("Error handling"): fallible Tauri commands return `Result<T, AppError>`.
//! `AppError` carries a stable `code` and a fixed, generic, user-safe
//! `message` — it never repeats a raw underlying error (SQL text, file
//! paths, driver-specific detail) back to the frontend. That detail is
//! logged to stderr at the point each conversion happens, for local
//! debugging only.

use serde::Serialize;

#[derive(Debug, thiserror::Error)]
pub enum AppError {
    /// The local application-data directory or database file could not be
    /// prepared (resolving/creating the directory, opening the file,
    /// acquiring a pooled connection).
    #[error("Local application storage is unavailable. Try restarting GoLive.")]
    Storage,

    /// A database read/write failed.
    #[error("A local database operation failed.")]
    Database,

    /// Schema migrations failed to apply.
    #[error("GoLive's local database could not be prepared.")]
    Migration,
}

impl AppError {
    fn code(&self) -> &'static str {
        match self {
            Self::Storage => "storage_unavailable",
            Self::Database => "database_error",
            Self::Migration => "migration_error",
        }
    }
}

/// Tauri serializes command `Err` values and sends them to the frontend as
/// JSON, so `AppError` must implement `Serialize`. It always serializes to
/// `{ "code": "...", "message": "..." }` — never the raw Rust error.
impl Serialize for AppError {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeStruct;
        let mut state = serializer.serialize_struct("AppError", 2)?;
        state.serialize_field("code", self.code())?;
        state.serialize_field("message", &self.to_string())?;
        state.end()
    }
}

impl From<std::io::Error> for AppError {
    fn from(err: std::io::Error) -> Self {
        eprintln!("[golive] filesystem error while preparing local storage: {err}");
        AppError::Storage
    }
}

impl From<r2d2::Error> for AppError {
    fn from(err: r2d2::Error) -> Self {
        eprintln!("[golive] database connection pool error: {err}");
        AppError::Database
    }
}

impl From<rusqlite::Error> for AppError {
    fn from(err: rusqlite::Error) -> Self {
        eprintln!("[golive] sqlite error: {err}");
        AppError::Database
    }
}
