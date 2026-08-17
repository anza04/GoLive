use serde::Serialize;

/// A GoLive project: the top-level container for a single migration/
/// documentation engagement. Everything TASK-006+ adds (Processes,
/// Captures, Screenshots, Recordings, generated documentation) will
/// belong to one of these via a `project_id` foreign key once those
/// tables exist — not implemented yet.
#[derive(Debug, Clone, Serialize)]
pub struct Project {
    pub id: String,
    pub name: String,
    pub description: String,
    /// Unix epoch milliseconds (UTC). Set once at creation, never changed.
    pub created_at: i64,
    /// Unix epoch milliseconds (UTC). Bumped whenever the project changes.
    pub updated_at: i64,
}
