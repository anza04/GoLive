//! `ProcessVersion` (TASK-018, editable since TASK-019): one persisted
//! AI-generated structured draft (see `ai::ProcessDraft`) for a
//! Process. See docs/architecture.md ("Process draft persistence") and
//! `migrations/0005_process_versions.sql`/`0006_process_versions_editable.sql`
//! for the full rationale.

use crate::ai::ProcessDraft;
use serde::Serialize;

#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct ProcessVersion {
    pub id: String,
    pub process_id: String,
    pub content: ProcessDraft,
    /// Unix epoch milliseconds (UTC). Set once at creation, never
    /// changed — same convention as every other model's `created_at`.
    pub created_at: i64,
    /// Unix epoch milliseconds (UTC). Bumped only when a user edits and
    /// saves this version's content (TASK-019) — regenerating never
    /// touches an existing row's `updated_at`, it inserts a brand new
    /// row with its own `created_at`/`updated_at` instead (SS5's rule).
    pub updated_at: i64,
}
