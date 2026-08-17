import { invoke } from "@tauri-apps/api/core";

export interface LocalStorageStatus {
  ready: boolean;
  /** Unix timestamp (seconds), kept only for an optional debug tooltip —
   * never shown as primary UI text. */
  initializedAt: string;
}

interface RawLocalStorageStatus {
  ready: boolean;
  initialized_at: string;
}

/**
 * Confirms local SQLite persistence is working by asking the backend to
 * ensure its one-time storage marker exists and return it — proving the
 * full React -> Tauri -> repository -> SQLite round trip without exposing
 * any database implementation detail (no file paths, no SQL).
 */
export async function getLocalStorageStatus(): Promise<LocalStorageStatus> {
  const raw = await invoke<RawLocalStorageStatus>("get_local_storage_status");
  return { ready: raw.ready, initializedAt: raw.initialized_at };
}
