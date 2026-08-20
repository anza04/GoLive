mod active_process;
mod commands;
mod db;
mod errors;
mod hotkey;
mod media;
mod models;
mod native;
mod recording;
mod repositories;
mod services;
mod tray;

use tauri::Manager;
use tauri_plugin_global_shortcut::{GlobalShortcutExt, ShortcutState};

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .setup(|app| {
            let data_dir = app.path().app_data_dir().map_err(|err| {
                eprintln!("[golive] failed to resolve app data directory: {err}");
                err
            })?;
            let db_service = db::DbService::init(&data_dir)?;
            let media_storage = media::MediaStorage::init(&data_dir)?;

            // Best-effort startup cleanup of screenshot media orphaned by
            // the Project/Process database cascade (see
            // docs/architecture.md, "Cascade media cleanup"): SQLite's
            // `ON DELETE CASCADE` removes Capture rows when their Process
            // (or Project) is deleted, but never knew about their PNG
            // files, so a sweep is needed to catch up. A failure here is
            // logged, never fatal — it must not block the application
            // from starting.
            let reconcile_service = services::capture::CaptureService::new(
                Box::new(repositories::capture::SqliteCaptureRepository::new(db_service.pool())),
                Box::new(repositories::process::SqliteProcessRepository::new(db_service.pool())),
                media_storage.clone(),
                Box::new(native::screenshot::WindowsScreenshotEngine),
            );
            match reconcile_service.reconcile_media() {
                Ok(removed) if removed > 0 => {
                    eprintln!("[golive] removed {removed} orphaned screenshot file(s) at startup");
                }
                Ok(_) => {}
                Err(err) => eprintln!("[golive] media reconciliation failed at startup: {err}"),
            }

            app.manage(db_service);
            app.manage(media_storage);
            app.manage(active_process::ActiveProcessState::default());
            app.manage(recording::RecordingState::default());

            // System tray (TASK-010): built last, once the state above
            // is settled, so its "Open GoLive" handler always has a
            // fully initialized app to reopen into.
            let tray_handles = tray::build(app)?;
            app.manage(tray_handles);

            // Global keyboard shortcut (TASK-011): registered here, not
            // as a top-level `.plugin(...)` before `.setup()`, matching
            // the plugin's own documented pattern. `hotkey::shortcut()`
            // is a small, pure, deterministic constructor — called fresh
            // on each side (handler comparison, registration) rather
            // than captured/cloned, since there's nothing to save by
            // sharing one instance.
            //
            // Registration failure (most likely: another already-running
            // application claimed the same combination first) is logged
            // and otherwise ignored — it must NOT prevent GoLive from
            // starting. This was not a theoretical concern: it was
            // caught during this task's own manual verification, when
            // the originally-chosen shortcut turned out to already be
            // claimed on the test machine and (before this was fixed)
            // took the entire application down at startup.
            if let Err(err) = app.handle().plugin(
                tauri_plugin_global_shortcut::Builder::new()
                    .with_handler(|app, pressed, event| {
                        if event.state() == ShortcutState::Pressed && pressed == &hotkey::shortcut() {
                            hotkey::handle_capture_shortcut(app);
                        }
                    })
                    .build(),
            ) {
                eprintln!("[golive] failed to initialize the global-shortcut plugin: {err}");
            } else if let Err(err) = app.global_shortcut().register(hotkey::shortcut()) {
                eprintln!(
                    "[golive] failed to register the global capture shortcut (already in use by \
                     another application?): {err}"
                );
            }

            // Bugfix, belt-and-suspenders with `tauri.conf.json`'s widget
            // window `"backgroundColor": [0, 0, 0, 0]` (see
            // DECISIONS.md): that config field was reported to still
            // leave the collapsed dot on an opaque rectangle. Per
            // Tauri's own `WebviewWindow::set_background_color` doc
            // (docs.rs, this app's pinned tauri version), a webview
            // window's background is actually painted in *three*
            // separate layers on Windows — the native window, the
            // WebView2 control sitting inside it, and the page's own
            // CSS — and `tauri.conf.json`'s window-level config field
            // is not guaranteed to reach the middle (WebView2/webview)
            // layer for a window declared with the "windows" array's
            // implicit-default-webview shorthand (no explicit
            // "webviews" array) the way this app's widget window is.
            // Setting it again here, explicitly, on the `WebviewWindow`
            // handle itself, targets that layer directly regardless of
            // how the declarative config routed it.
            if let Some(widget_window) = app.get_webview_window("widget") {
                if let Err(err) = widget_window.set_background_color(Some(tauri::webview::Color(0, 0, 0, 0))) {
                    eprintln!("[golive] failed to set widget webview background color: {err}");
                }
            } else {
                eprintln!("[golive] widget window not found during setup — cannot set its background color");
            }

            Ok(())
        })
        // Hides a window instead of letting it close — applies to both
        // "main" and "widget" (TASK-011): neither should ever be truly
        // destroyed while the app is running, since either one might be
        // the user's only visible way back into a hidden GoLive. The
        // tray's "Quit" item (see `tray::build`) is the one real exit,
        // plus a normal window-manager force-close.
        .on_window_event(|window, event| {
            if let tauri::WindowEvent::CloseRequested { api, .. } = event {
                api.prevent_close();
                if let Err(err) = window.hide() {
                    eprintln!("[golive] failed to hide window {} on close: {err}", window.label());
                }
            }
        })
        .invoke_handler(tauri::generate_handler![
            commands::foundation::check_foundation_status,
            commands::storage::get_local_storage_status,
            commands::project::create_project,
            commands::project::list_projects,
            commands::project::get_project,
            commands::project::update_project,
            commands::project::delete_project,
            commands::process::create_process,
            commands::process::list_processes,
            commands::process::get_process,
            commands::process::update_process,
            commands::process::delete_process,
            commands::capture::create_capture,
            commands::capture::list_captures,
            commands::capture::get_capture,
            commands::capture::update_capture,
            commands::capture::delete_capture,
            commands::capture::create_screenshot_capture,
            commands::capture::get_capture_media,
            commands::capture::get_recording_media,
            commands::active_process::sync_active_process,
            commands::active_process::get_active_process,
            commands::widget::hide_widget,
            commands::widget::set_widget_expanded,
            commands::recording::start_recording_capture,
            commands::recording::stop_recording_capture,
            commands::recording::get_recording_status,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
