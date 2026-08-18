//! Native, platform-specific functionality (see docs/architecture.md,
//! "Native Windows functionality boundary"). `screenshot` (TASK-009) is
//! the first real occupant — future recording/microphone/hotkey work
//! belongs here too, each behind its own small trait, never leaking
//! platform detail into `services`, `commands`, or the frontend.

pub mod screenshot;
