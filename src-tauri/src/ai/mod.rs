//! The AI boundary docs/architecture.md §8 sketches, given its first
//! real occupant (TASK-017): a Rust trait (`AiService`) the rest of the
//! app depends on, never on OpenAI-specific types — a future second
//! provider is a new `impl AiService`, not a change to any caller. See
//! `ai::openai::OpenAiService` for the one implementation that exists
//! today.
//!
//! Scope is deliberately narrow, matching roadmap.md TASK-017: prove
//! the round trip (a Process's Captures in, a structured draft out)
//! before anything persists it (TASK-018) or lets a user edit it
//! (TASK-019).

pub mod openai;

use crate::errors::AppError;
use serde::{Deserialize, Serialize};

/// Everything `AiService::generate_process_draft` needs about one
/// Capture to describe it to the model — not the full `models::capture::Capture`
/// (which has no room for the actual screenshot bytes, and carries
/// `process_id`/timestamps the model has no use for). Built by
/// `services::process_draft::ProcessDraftService`, which is the one
/// place that reads capture metadata (`CaptureRepository`) and
/// screenshot bytes (`MediaStorage`) and assembles them into this.
pub struct CaptureForAi {
    pub id: String,
    /// One of `"screenshot"`, `"recording"`, `"note"` — the model gets
    /// the plain wire string, not the Rust `CaptureType` enum; nothing
    /// here needs to be an app-internal type.
    pub capture_type: String,
    pub title: String,
    pub description: String,
    /// `Some(png_bytes)` only for a screenshot Capture with real media
    /// on disk — `None` for recording/note Captures (no image exists),
    /// and also `None` if a screenshot Capture's PNG is unexpectedly
    /// missing (treated as "describe it from title/description alone"
    /// rather than failing the whole generation over one capture).
    pub screenshot_png: Option<Vec<u8>>,
}

/// Input to `AiService::generate_process_draft` — a Process's identity
/// plus its Captures, already gathered and ordered (chronologically,
/// oldest first — the order the underlying work actually happened in,
/// not the app's own `updated_at DESC` list order) by the caller.
pub struct ProcessDraftRequest {
    pub process_name: String,
    pub process_description: String,
    pub captures: Vec<CaptureForAi>,
}

/// The AI-structured result — a first cut at the "structured, editable
/// business process" README.md's product description promises.
/// Deliberately minimal for TASK-017 (a summary plus an ordered list of
/// steps): TASK-018 owns deciding whether this needs to grow before it
/// becomes the real persisted shape.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ProcessDraft {
    pub summary: String,
    pub steps: Vec<ProcessDraftStep>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ProcessDraftStep {
    pub title: String,
    pub description: String,
    /// Which Capture(s) (by id) this step is based on — TASK-020's
    /// "embed screenshots referenced by the relevant steps" needs this
    /// link to exist from the start; retrofitting it in TASK-018/019
    /// would mean re-deriving it from scratch with no model context
    /// available anymore. May be empty (a step the model synthesized
    /// from general context rather than one specific capture) — never
    /// an id that isn't one of the Captures actually sent in the
    /// request (see `ai::openai::OpenAiService`, which validates this).
    pub capture_ids: Vec<String>,
}

/// The AI provider abstraction (docs/architecture.md §8). Everything
/// above the trait (the command layer, `services::process_draft`) talks
/// only to this — never to an OpenAI-specific type or endpoint.
pub trait AiService: Send + Sync {
    /// `api_key` is supplied per call, not held by the implementor — the
    /// same "read the secret only for as long as one call needs it"
    /// shape `services::settings::SettingsService::test_connection`
    /// already established (see credentials.rs).
    fn generate_process_draft(&self, api_key: &str, request: &ProcessDraftRequest) -> Result<ProcessDraft, AppError>;
}
