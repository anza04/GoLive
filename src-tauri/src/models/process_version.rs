//! `ProcessVersion` (TASK-018): one persisted, immutable AI-generated
//! structured draft (see `ai::ProcessDraft`) for a Process. See
//! docs/architecture.md ("Process draft persistence") and
//! `migrations/0005_process_versions.sql` for the full rationale.

use crate::ai::ProcessDraft;
use serde::Serialize;

#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct ProcessVersion {
    pub id: String,
    pub process_id: String,
    pub content: ProcessDraft,
    /// Unix epoch milliseconds (UTC). Set once at creation — a
    /// ProcessVersion is never edited in place at this stage, so unlike
    /// every other model in this app there is no `updated_at`.
    pub created_at: i64,
}
