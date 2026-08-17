mod commands;
mod db;
mod errors;
mod models;
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
            app.manage(db_service);
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
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
