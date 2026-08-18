//! Commands for the cross-window active-process mirror (TASK-011) and
//! (still) the tray label it feeds (see `tray.rs`). Supersedes
//! TASK-010's narrower `set_active_process_tray`: same trigger point in
//! the frontend (`stores/activeProcess.tsx`), now carrying full identity
//! (ids, not just display names) so the floating widget (and the global
//! shortcut, see `hotkey.rs`) can actually act on the active Process,
//! not just display it.

use crate::active_process::{ActiveProcessInfo, ActiveProcessState, ACTIVE_PROCESS_CHANGED_EVENT};
use crate::tray::TrayHandles;
use tauri::{AppHandle, Emitter, State};

/// Updates the shared active-process mirror, the tray label, and
/// broadcasts the change to every window (the floating widget listens —
/// see docs/architecture.md, §23/§24). `input: None` clears it.
/// Infallible, same rationale as TASK-010's original command: nothing
/// user-facing needs to react to this failing.
#[tauri::command]
pub fn sync_active_process(
    input: Option<ActiveProcessInfo>,
    state: State<ActiveProcessState>,
    tray: State<TrayHandles>,
    app: AppHandle,
) {
    state.set(input.clone());

    let label = input
        .as_ref()
        .map(|active| format!("{} — {}", active.process_name, active.project_name));
    tray.set_active_process(label.as_deref());

    if let Err(err) = app.emit(ACTIVE_PROCESS_CHANGED_EVENT, input) {
        eprintln!("[golive] failed to broadcast active-process change: {err}");
    }
}

/// Lets a window that wasn't open (or wasn't listening yet) when the
/// last change happened — e.g. the floating widget, just shown after
/// being hidden — fetch the current value on mount instead of waiting
/// for the next change.
#[tauri::command]
pub fn get_active_process(state: State<ActiveProcessState>) -> Option<ActiveProcessInfo> {
    state.get()
}
