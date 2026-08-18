//! In-memory mirror of "the active Process" (TASK-011), shared across
//! windows. The frontend's `stores/activeProcess.tsx` (in the main
//! window) is the source of truth for *setting* it, but Tauri windows
//! are separate webview processes with no shared JS memory — the
//! floating capture widget (a second window, see `tauri.conf.json`)
//! can't read the main window's React Context directly. This tiny piece
//! of Tauri-managed state, updated via
//! `commands::active_process::sync_active_process` and broadcast via an
//! app-wide event, is how the widget (and the global-shortcut handler,
//! see `hotkey.rs`) stay in sync — see docs/architecture.md,
//! "Cross-window active-process sync".
//!
//! Never persisted — resets to `None` on every launch, same as the
//! frontend store it mirrors.

use serde::{Deserialize, Serialize};
use std::sync::Mutex;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActiveProcessInfo {
    pub process_id: String,
    pub process_name: String,
    pub project_id: String,
    pub project_name: String,
}

/// Tauri-managed state: the last value pushed by `sync_active_process`.
#[derive(Default)]
pub struct ActiveProcessState(pub Mutex<Option<ActiveProcessInfo>>);

impl ActiveProcessState {
    /// Reads the current value. Recovers from a poisoned lock (a prior
    /// holder panicking while holding it) rather than panicking itself —
    /// there is no expected path that poisons this lock, but this is
    /// cheap insurance consistent with this project's "no unwrap on a
    /// production path" convention.
    pub fn get(&self) -> Option<ActiveProcessInfo> {
        self.0.lock().unwrap_or_else(|poisoned| poisoned.into_inner()).clone()
    }

    pub fn set(&self, value: Option<ActiveProcessInfo>) {
        *self.0.lock().unwrap_or_else(|poisoned| poisoned.into_inner()) = value;
    }
}

/// Emitted (via `AppHandle::emit`, Rust-side — not gated by the frontend
/// ACL) to every window whenever the active Process changes. Payload is
/// `Option<ActiveProcessInfo>` — `null` means cleared.
pub const ACTIVE_PROCESS_CHANGED_EVENT: &str = "active-process-changed";

#[cfg(test)]
mod tests {
    use super::*;

    fn sample() -> ActiveProcessInfo {
        ActiveProcessInfo {
            process_id: "p1".to_string(),
            process_name: "Create a sales order".to_string(),
            project_id: "proj1".to_string(),
            project_name: "ERP Migration".to_string(),
        }
    }

    #[test]
    fn starts_empty() {
        let state = ActiveProcessState::default();
        assert!(state.get().is_none());
    }

    #[test]
    fn set_then_get_round_trips() {
        let state = ActiveProcessState::default();
        state.set(Some(sample()));
        assert_eq!(state.get().unwrap().process_id, "p1");
    }

    #[test]
    fn set_none_clears_a_previously_set_value() {
        let state = ActiveProcessState::default();
        state.set(Some(sample()));
        state.set(None);
        assert!(state.get().is_none());
    }
}
