mod commands;
mod db;
mod errors;
mod repositories;

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
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
