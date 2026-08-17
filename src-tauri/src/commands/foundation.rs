/// Minimal proof-of-life command used by the frontend to confirm that the
/// React -> Tauri -> Rust pipeline is wired correctly. Carries no business
/// logic. Moved here unchanged from `lib.rs` (TASK-001) now that a
/// `commands/` module exists to hold it (TASK-004).
#[tauri::command]
pub fn check_foundation_status() -> String {
    "Rust backend connected.".to_string()
}
