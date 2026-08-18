mod commands;
mod db;
mod errors;
mod media;
mod models;
mod native;
mod repositories;
mod services;

use tauri::Manager;

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
            Ok(())
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
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
