//! The floating capture widget's one window-management command
//! (TASK-011): its own "hide" button. Showing it is tray-only (see
//! `tray::toggle_widget`) — the widget has no reason to show itself.

use tauri::{AppHandle, Manager};

#[tauri::command]
pub fn hide_widget(app: AppHandle) {
    if let Some(window) = app.get_webview_window("widget") {
        if let Err(err) = window.hide() {
            eprintln!("[golive] failed to hide widget: {err}");
        }
    }
}
