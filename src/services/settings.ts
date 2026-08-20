import { invoke } from "@tauri-apps/api/core";

/**
 * TASK-016: the OpenAI API key lives in the Windows Credential Manager
 * (see docs/architecture.md §12) — this module never receives the key
 * back out once saved, only whether one is currently set. Mirrors the
 * "React never touches storage directly" convention every other
 * `services/*.ts` file follows (see `services/README.md`).
 */

/** Saves `apiKey` to the Windows Credential Manager, overwriting any
 * previously saved key. Rejects with the usual `AppError` shape (via
 * `getErrorMessage`) if it's empty or over the length limit. */
export async function saveApiKey(apiKey: string): Promise<void> {
  await invoke("save_api_key", { input: { api_key: apiKey } });
}

/** Whether an API key is currently saved — never the key itself. */
export async function hasApiKey(): Promise<boolean> {
  return invoke<boolean>("has_api_key");
}

/** Clears the saved API key. Clearing when none is saved is not an
 * error (same "already-gone is success" convention every delete
 * operation in this app follows). */
export async function clearApiKey(): Promise<void> {
  await invoke("clear_api_key");
}

/**
 * Tests the *currently saved* key against OpenAI — not whatever might
 * be sitting unsaved in a form field. Resolves on success; rejects with
 * an `AppError`-shaped reason on failure (no saved key, OpenAI rejected
 * it, or the request couldn't reach OpenAI at all).
 */
export async function testApiKeyConnection(): Promise<void> {
  await invoke("test_api_key_connection");
}
