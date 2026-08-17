use rusqlite::types::{FromSql, FromSqlError, FromSqlResult, ToSql, ToSqlOutput, ValueRef};
use serde::{Deserialize, Serialize};

/// A Process's lifecycle status. Represented as a real Rust enum in
/// business logic — never an arbitrary string — and persisted in SQLite /
/// sent over IPC as one of three stable lowercase strings. No automatic
/// transitions exist; the user changes status explicitly (see
/// docs/architecture.md, "Process status").
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ProcessStatus {
    Draft,
    InProgress,
    Completed,
}

impl ProcessStatus {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Draft => "draft",
            Self::InProgress => "in_progress",
            Self::Completed => "completed",
        }
    }

    /// Parses one of the three stable strings. Returns `None` for
    /// anything else — callers (the service layer) turn that into a safe
    /// `AppError::Validation`, never accepting an arbitrary status string.
    pub fn parse(value: &str) -> Option<Self> {
        match value {
            "draft" => Some(Self::Draft),
            "in_progress" => Some(Self::InProgress),
            "completed" => Some(Self::Completed),
            _ => None,
        }
    }
}

// Lets the repository bind/read `ProcessStatus` directly as a SQLite TEXT
// column (`params![process.status, ...]`, `row.get::<_, ProcessStatus>(n)`)
// instead of manually converting to/from `String` at every call site.
impl ToSql for ProcessStatus {
    fn to_sql(&self) -> rusqlite::Result<ToSqlOutput<'_>> {
        Ok(ToSqlOutput::from(self.as_str()))
    }
}

impl FromSql for ProcessStatus {
    fn column_result(value: ValueRef<'_>) -> FromSqlResult<Self> {
        ProcessStatus::parse(value.as_str()?).ok_or(FromSqlError::InvalidType)
    }
}

/// A business process within a Project, documenting one user workflow
/// (e.g. "Create a sales order"). The future parent/context for Captures,
/// screen/microphone recordings, transcripts, and AI-generated analysis —
/// none of which are implemented yet.
#[derive(Debug, Clone, Serialize)]
pub struct Process {
    pub id: String,
    pub project_id: String,
    pub name: String,
    pub description: String,
    pub status: ProcessStatus,
    /// Unix epoch milliseconds (UTC). Set once at creation, never changed.
    pub created_at: i64,
    /// Unix epoch milliseconds (UTC). Bumped whenever the process changes.
    pub updated_at: i64,
}
