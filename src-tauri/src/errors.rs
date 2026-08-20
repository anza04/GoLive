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
    /// The local application-data directory, database file, or captured-
    /// media directory/file could not be prepared or accessed (resolving/
    /// creating a directory, opening a file, acquiring a pooled
    /// connection). Reused by `media::MediaStorage` (TASK-009) for PNG
    /// read/write/delete I/O failures via `From<std::io::Error>` below —
    /// a filesystem failure storing captured media is the same category
    /// of problem as one preparing the database file.
    #[error("Local application storage is unavailable. Try restarting GoLive.")]
    Storage,

    /// A database read/write failed.
    #[error("A local database operation failed.")]
    Database,

    /// Schema migrations failed to apply.
    #[error("GoLive's local database could not be prepared.")]
    Migration,

    /// Input failed application-level validation (e.g. an empty required
    /// field, a value over its length limit). Unlike the other variants,
    /// the message here *is* meant to be shown to the user as-is — it's
    /// always an author-written, safe string describing what to fix, never
    /// a raw underlying error.
    #[error("{0}")]
    Validation(String),

    /// The requested record does not exist (e.g. `get`/`delete` by an id
    /// that isn't in the database).
    #[error("The requested item could not be found.")]
    NotFound,

    /// A native media capture operation failed — e.g. no display is
    /// available to screenshot, or the captured image could not be
    /// encoded (see TASK-009, `native::screenshot`). Like `Validation`,
    /// the message here is an author-written, safe, specific string
    /// shown to the user as-is — never a raw Windows API or image-crate
    /// error. Filesystem failures while *storing* captured media (as
    /// opposed to capturing it) reuse `Storage`/`Database` instead — see
    /// `media::MediaStorage`.
    #[error("{0}")]
    Capture(String),

    /// The OS credential store (Windows Credential Manager, TASK-016)
    /// could not be read from or written to — a genuinely different
    /// failure category from `Storage` (that's this app's own SQLite/
    /// filesystem area; this is a separate OS subsystem GoLive doesn't
    /// own). Like `Validation`/`Capture`, the message is an
    /// author-written, safe, specific string — never the raw `keyring`
    /// error, which could otherwise leak Windows API detail. See
    /// `credentials::CredentialStore`.
    #[error("{0}")]
    Credential(String),

    /// An outbound network call (TASK-016: testing the stored OpenAI API
    /// key) failed to reach its destination, timed out, or the API
    /// rejected it. Like `Capture`/`Credential`, an author-written safe
    /// string — never the raw `reqwest` error (which can include
    /// resolved IPs, redirect chains, etc.). See `ai::openai::test_api_key`.
    #[error("{0}")]
    Network(String),

    /// The AI provider's call *completed* (unlike `Network`, which means
    /// the call itself failed) but its response couldn't be turned into
    /// usable structured content — e.g. a refusal, or output that didn't
    /// match the requested JSON schema despite strict mode (TASK-017).
    /// Like `Capture`/`Credential`/`Network`, an author-written safe
    /// string — never the raw response body. See `ai::openai::OpenAiService`.
    #[error("{0}")]
    Ai(String),
}

impl AppError {
    fn code(&self) -> &'static str {
        match self {
            Self::Storage => "storage_unavailable",
            Self::Database => "database_error",
            Self::Migration => "migration_error",
            Self::Validation(_) => "validation_error",
            Self::NotFound => "not_found",
            Self::Capture(_) => "capture_error",
            Self::Credential(_) => "credential_error",
            Self::Network(_) => "network_error",
            Self::Ai(_) => "ai_error",
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
