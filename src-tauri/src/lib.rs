/// Minimal proof-of-life command used by the frontend to confirm that the
/// React -> Tauri -> Rust pipeline is wired correctly. Carries no business
/// logic; it will be superseded by real commands in later tasks.
#[tauri::command]
fn check_foundation_status() -> String {
    "Rust backend connected.".to_string()
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .invoke_handler(tauri::generate_handler![check_foundation_status])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
