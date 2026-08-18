//! Native, platform-specific functionality (see docs/architecture.md,
//! "Native Windows functionality boundary"). `screenshot` (TASK-009) was
//! the first real occupant; `recording` (TASK-013) is the second —
//! future microphone/hotkey work belongs here too, each behind its own
//! small trait, never leaking platform detail into `services`,
//! `commands`, or the frontend.

pub mod recording;
pub mod screenshot;
