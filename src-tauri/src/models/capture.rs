use rusqlite::types::{FromSql, FromSqlError, FromSqlResult, ToSql, ToSqlOutput, ValueRef};
use serde::{Deserialize, Serialize};

/// The kind of evidence a Capture represents. A real Rust enum in
/// business logic — never an arbitrary string — persisted in SQLite /
/// sent over IPC as one of three stable lowercase strings, same pattern
/// as `ProcessStatus` (see `models::process`). "screenshot" and
/// "recording" describe the *future* capture type — no actual
/// screenshot/recording media exists yet (see docs/architecture.md,
/// "Capture domain"); a Capture is metadata only.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CaptureType {
    Screenshot,
    Recording,
    Note,
}

impl CaptureType {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Screenshot => "screenshot",
            Self::Recording => "recording",
            Self::Note => "note",
        }
    }

    /// Parses one of the three stable strings. Returns `None` for
    /// anything else — callers (the service layer) turn that into a safe
    /// `AppError::Validation`, never accepting an arbitrary type string.
    pub fn parse(value: &str) -> Option<Self> {
        match value {
            "screenshot" => Some(Self::Screenshot),
            "recording" => Some(Self::Recording),
            "note" => Some(Self::Note),
            _ => None,
        }
    }
}

// Lets the repository bind/read `CaptureType` directly as a SQLite TEXT
// column (`params![capture.capture_type, ...]`,
// `row.get::<_, CaptureType>(n)`) instead of manually converting to/from
// `String` at every call site.
impl ToSql for CaptureType {
    fn to_sql(&self) -> rusqlite::Result<ToSqlOutput<'_>> {
        Ok(ToSqlOutput::from(self.as_str()))
    }
}

impl FromSql for CaptureType {
    fn column_result(value: ValueRef<'_>) -> FromSqlResult<Self> {
        CaptureType::parse(value.as_str()?).ok_or(FromSqlError::InvalidType)
    }
}

/// A single piece of evidence/content collected while documenting a
/// Process — a screenshot, a recording, or a free-form note. Metadata
/// only for now: no actual screenshot/recording/media file is attached
/// yet (see docs/architecture.md, "Capture domain") — that's later work.
#[derive(Debug, Clone, Serialize)]
pub struct Capture {
    pub id: String,
    pub process_id: String,
    // `type` is a reserved word in Rust, so the field is named
    // `capture_type` — renamed to the wire key "type" so it still
    // matches the model's documented shape (`Capture { ..., type, ... }`)
    // and the frontend's `Capture.type`.
    #[serde(rename = "type")]
    pub capture_type: CaptureType,
    pub title: String,
    pub description: String,
    /// Unix epoch milliseconds (UTC). Set once at creation, never changed.
    pub created_at: i64,
    /// Unix epoch milliseconds (UTC). Bumped whenever the capture changes.
    pub updated_at: i64,
}
