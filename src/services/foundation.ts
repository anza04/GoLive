import { invoke } from "@tauri-apps/api/core";

/**
 * Confirms the React -> Tauri -> Rust pipeline is wired correctly by
 * calling the backend's proof-of-life command.
 *
 * This is the reference example for the app's Tauri communication
 * convention: React components never call `invoke()` directly — they go
 * through a service function like this one instead. See
 * docs/architecture.md ("Frontend -> Tauri communication") for the full
 * convention, including where feature-specific service functions belong.
 */
export function checkFoundationStatus(): Promise<string> {
  return invoke<string>("check_foundation_status");
}
