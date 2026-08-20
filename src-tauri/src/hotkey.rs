//! Global keyboard shortcut (TASK-011): lets the user trigger a
//! screenshot capture into the active Process from anywhere in Windows,
//! without focusing any GoLive window. Registered once from `.setup()`
//! (see `lib.rs`); the shortcut itself is hardcoded for now — no
//! customization UI exists yet (see docs/architecture.md).
//!
//! Deliberately handled entirely in Rust rather than bouncing through a
//! window's JS: a global shortcut has no "requesting window" the way a
//! button click does, and every dependency this needs — the active
//! Process (`active_process::ActiveProcessState`) and the same
//! `CaptureService::create_screenshot` TASK-009 already built — is
//! already reachable from here. The one place the result surfaces in
//! the UI is the floating widget, which listens for
//! `SCREENSHOT_CAPTURED_EVENT` rather than driving the capture itself.

use crate::active_process::ActiveProcessState;
use crate::db::DbService;
use crate::media::MediaStorage;
use crate::native::screenshot::WindowsScreenshotEngine;
use crate::repositories::capture::SqliteCaptureRepository;
use crate::repositories::process::SqliteProcessRepository;
use crate::services::capture::CaptureService;
use serde::Serialize;
use tauri::{AppHandle, Emitter, Manager};
use tauri_plugin_global_shortcut::{Code, Modifiers, Shortcut};

pub const SCREENSHOT_CAPTURED_EVENT: &str = "screenshot-captured";

/// A plain, fixed title — the hotkey is a no-dialog, instant action
/// (see docs/architecture.md, "Global hotkey"); the Capture's created/
/// updated dates already disambiguate multiple screenshots in the UI,
/// the same way TASK-012's planned quick markers will.
const HOTKEY_CAPTURE_TITLE: &str = "Screenshot";

#[derive(Serialize, Clone)]
#[serde(tag = "status", rename_all = "snake_case")]
enum CaptureResult {
    Ok,
    Error { message: String },
    NoActiveProcess,
}

/// The one shortcut TASK-011 registers. Not user-configurable yet (see
/// roadmap.md — hotkey customization UI is explicitly out of scope). A
/// three-modifier combination was chosen specifically to minimize the
/// chance of colliding with a shortcut some other, already-running
/// application registered first — see `lib.rs`, where registration
/// failure (this OS refusing an already-claimed combination) is handled
/// gracefully rather than treated as a fatal startup error, since no
/// combination can be guaranteed collision-free on every machine.
pub fn shortcut() -> Shortcut {
    Shortcut::new(Some(Modifiers::CONTROL | Modifiers::ALT | Modifiers::SHIFT), Code::KeyS)
}

/// Called on every press of the registered shortcut (see `lib.rs`'s
/// plugin handler). Captures into whichever Process
/// `ActiveProcessState` currently holds; if there isn't one, tells the
/// widget why instead of silently doing nothing.
pub fn handle_capture_shortcut(app: &AppHandle) {
    let active = app.state::<ActiveProcessState>().get();

    let Some(active) = active else {
        emit_result(app, CaptureResult::NoActiveProcess);
        return;
    };

    let db = app.state::<DbService>();
    let media = app.state::<MediaStorage>();
    let service = CaptureService::new(
        Box::new(SqliteCaptureRepository::new(db.pool())),
        Box::new(SqliteProcessRepository::new(db.pool())),
        media.inner().clone(),
        Box::new(WindowsScreenshotEngine),
    );

    let result = match service.create_screenshot(&active.process_id, HOTKEY_CAPTURE_TITLE, None) {
        Ok(capture) => {
            // Bugfix (see DECISIONS.md): this path calls `CaptureService`
            // directly rather than going through the `create_screenshot_capture`
            // Tauri command (see `commands::capture`, "no requesting
            // window" doc comment above) — which meant it never ran that
            // command's `CAPTURE_CREATED_EVENT` emit, so a hotkey
            // screenshot never appeared in an already-open main-window
            // Captures section until it was remounted. The widget's own
            // `SCREENSHOT_CAPTURED_EVENT` (emitted below via
            // `emit_result`) only ever told the widget "ok/error" — it
            // carries no Capture payload a list could splice in.
            if let Err(err) = app.emit(crate::commands::capture::CAPTURE_CREATED_EVENT, &capture) {
                eprintln!("[golive] failed to broadcast capture creation: {err}");
            }
            CaptureResult::Ok
        }
        Err(err) => CaptureResult::Error { message: err.to_string() },
    };

    emit_result(app, result);
}

fn emit_result(app: &AppHandle, result: CaptureResult) {
    if let Err(err) = app.emit(SCREENSHOT_CAPTURED_EVENT, result) {
        eprintln!("[golive] failed to broadcast screenshot-capture result: {err}");
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // Pins the exact wire shape the widget's frontend listener parses
    // (see `services/captures.ts`, `onScreenshotCaptured`) — a silent
    // rename/shape change here would break that parsing without a
    // compile error on either side.
    #[test]
    fn capture_result_serializes_with_a_status_tag() {
        assert_eq!(serde_json::to_string(&CaptureResult::Ok).unwrap(), r#"{"status":"ok"}"#);
        assert_eq!(
            serde_json::to_string(&CaptureResult::Error { message: "oops".to_string() }).unwrap(),
            r#"{"status":"error","message":"oops"}"#
        );
        assert_eq!(
            serde_json::to_string(&CaptureResult::NoActiveProcess).unwrap(),
            r#"{"status":"no_active_process"}"#
        );
    }

    #[test]
    fn shortcut_is_deterministic() {
        assert_eq!(shortcut(), shortcut());
    }
}
