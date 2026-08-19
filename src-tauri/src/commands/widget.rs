//! The floating capture widget's window-management commands: hiding it
//! entirely (TASK-011 — the tray is the only way to bring it back, see
//! `tray::toggle_widget`), and resizing it between its two in-window
//! states (TASK-014's fix): a small draggable "dot" (the resting state)
//! and the full expanded panel. Resizing programmatically like this
//! (rather than exposing `@tauri-apps/api/window`'s resize API to the
//! widget's JS) needs no capability grant — the same "custom app-defined
//! commands aren't ACL-gated" reasoning `hide_widget` itself already
//! relies on (see DECISIONS.md, TASK-011).

use tauri::{AppHandle, LogicalSize, Manager};

/// The widget's resting, collapsed size — small enough to read as a
/// single draggable dot, not a window (see `widget.css`, `.widget--dot`).
const DOT_SIZE: (f64, f64) = (56.0, 56.0);

/// The widget's size once expanded (its original TASK-011/012/013
/// panel size) — wide/tall enough for the header, active-Process
/// summary, and three action buttons.
const EXPANDED_SIZE: (f64, f64) = (260.0, 250.0);

#[tauri::command]
pub fn hide_widget(app: AppHandle) {
    if let Some(window) = app.get_webview_window("widget") {
        if let Err(err) = window.hide() {
            eprintln!("[golive] failed to hide widget: {err}");
        }
    }
}

/// Resizes the widget window between its dot and expanded sizes.
/// `tauri.conf.json`'s `"resizable": false` only disables user-driven
/// resize via window chrome (there is none — `decorations: false`); it
/// does not block a programmatic `set_size` call like this one.
#[tauri::command]
pub fn set_widget_expanded(expanded: bool, app: AppHandle) {
    let Some(window) = app.get_webview_window("widget") else {
        eprintln!("[golive] widget window not found while resizing");
        return;
    };

    let (width, height) = if expanded { EXPANDED_SIZE } else { DOT_SIZE };
    if let Err(err) = window.set_size(LogicalSize::new(width, height)) {
        eprintln!("[golive] failed to resize widget window: {err}");
    }
}
