//! System tray icon: lets GoLive keep running with its main window
//! hidden, and gives the user a real way to reopen/quit it and to
//! show/hide the floating capture widget (TASK-011) — see
//! docs/architecture.md, "Background persistence and system tray".
//! Deliberately minimal: one icon, a four-item menu — not a settings
//! surface of its own.

use tauri::menu::{Menu, MenuItem};
use tauri::tray::{TrayIcon, TrayIconBuilder};
use tauri::{App, Manager, Wry};

/// Handles kept in Tauri-managed state so `commands::active_process` can
/// update the tray after it's built — `TrayIconBuilder::build` only
/// runs once, in `.setup()`, but which Process is "active" (and
/// therefore what the tray should show) changes throughout the app's
/// lifetime.
pub struct TrayHandles {
    icon: TrayIcon<Wry>,
    active_label: MenuItem<Wry>,
}

impl TrayHandles {
    /// Reflects the current active-process label in both the tray's
    /// hover tooltip and its menu's informational item — updated live,
    /// no menu rebuild needed. `None` means no Process is currently
    /// active. Infallible: a failure to update a native tray label is
    /// logged and otherwise ignored — nothing user-facing depends on it
    /// succeeding (see `commands::active_process::sync_active_process`).
    pub fn set_active_process(&self, label: Option<&str>) {
        let (tooltip, menu_text) = tray_texts(label);
        if let Err(err) = self.icon.set_tooltip(Some(&tooltip)) {
            eprintln!("[golive] failed to update tray tooltip: {err}");
        }
        if let Err(err) = self.active_label.set_text(&menu_text) {
            eprintln!("[golive] failed to update tray menu label: {err}");
        }
    }
}

/// Pure formatting, pulled out of `set_active_process` so it can be unit
/// tested without a real tray icon (native tray/menu handles can't be
/// constructed outside a running Tauri app).
fn tray_texts(label: Option<&str>) -> (String, String) {
    match label {
        Some(label) => (format!("GoLive — {label}"), format!("Active: {label}")),
        None => ("GoLive".to_string(), "No active process".to_string()),
    }
}

/// Shows the floating widget if hidden, hides it if shown. Used by both
/// the tray's "Toggle Widget" menu item and — nowhere else yet, but
/// kept as a plain function rather than inlined in the menu-event
/// closure so a future hotkey/entry point could reuse it.
fn toggle_widget(app: &tauri::AppHandle) {
    let Some(widget) = app.get_webview_window("widget") else {
        eprintln!("[golive] widget window not found");
        return;
    };
    let is_visible = widget.is_visible().unwrap_or(false);
    let result = if is_visible { widget.hide() } else { widget.show() };
    if let Err(err) = result {
        eprintln!("[golive] failed to toggle widget visibility: {err}");
    }
}

/// Builds the tray icon and its menu, and wires the menu's click
/// handling ("Open GoLive" shows/focuses the main window; "Toggle
/// Widget" shows/hides the floating capture widget, TASK-011; "Quit" is
/// the only real exit besides a window-manager force-close — see
/// `lib.rs`'s `on_window_event`, which hides rather than closes the main
/// window). Called once from `.setup()`.
pub fn build(app: &App<Wry>) -> tauri::Result<TrayHandles> {
    let active_label = MenuItem::with_id(app, "active_process", "No active process", false, None::<&str>)?;
    let open_item = MenuItem::with_id(app, "open", "Open GoLive", true, None::<&str>)?;
    let toggle_widget_item = MenuItem::with_id(app, "toggle_widget", "Toggle Widget", true, None::<&str>)?;
    let quit_item = MenuItem::with_id(app, "quit", "Quit", true, None::<&str>)?;
    let menu = Menu::with_items(app, &[&active_label, &open_item, &toggle_widget_item, &quit_item])?;

    let mut builder = TrayIconBuilder::new()
        .tooltip("GoLive")
        .menu(&menu)
        .show_menu_on_left_click(true)
        .on_menu_event(|app, event| match event.id.as_ref() {
            "open" => {
                if let Some(window) = app.get_webview_window("main") {
                    let _ = window.show();
                    let _ = window.set_focus();
                }
            }
            "toggle_widget" => toggle_widget(app),
            "quit" => app.exit(0),
            _ => {}
        });

    if let Some(icon) = app.default_window_icon() {
        builder = builder.icon(icon.clone());
    }

    let icon = builder.build(app)?;

    Ok(TrayHandles { icon, active_label })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tray_texts_for_an_active_process_names_it_in_both_strings() {
        let (tooltip, menu_text) = tray_texts(Some("Create a sales order — ERP Migration"));
        assert_eq!(tooltip, "GoLive — Create a sales order — ERP Migration");
        assert_eq!(menu_text, "Active: Create a sales order — ERP Migration");
    }

    #[test]
    fn tray_texts_for_no_active_process_says_so_plainly() {
        let (tooltip, menu_text) = tray_texts(None);
        assert_eq!(tooltip, "GoLive");
        assert_eq!(menu_text, "No active process");
    }
}
