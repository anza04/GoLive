import { invoke } from "@tauri-apps/api/core";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";

/**
 * Which Process is currently "active" — mirrored into Rust
 * (`active_process::ActiveProcessState`) so windows other than the main
 * one (the floating capture widget, TASK-011) can read/react to it too,
 * since Tauri windows are separate webview runtimes with no shared JS
 * memory (see docs/architecture.md, "Cross-window active-process
 * sync"). The main window's `stores/activeProcess.tsx` is the source of
 * truth for *setting* this; this service is how it's pushed to Rust and
 * how any window (including the main one, in principle) can read it
 * back or subscribe to changes.
 */
export interface ActiveProcessInfo {
  processId: string;
  processName: string;
  projectId: string;
  projectName: string;
}

const ACTIVE_PROCESS_CHANGED_EVENT = "active-process-changed";

// The IPC wire shape: matches the Rust `ActiveProcessInfo` struct's
// field names exactly (snake_case).
interface RawActiveProcessInfo {
  process_id: string;
  process_name: string;
  project_id: string;
  project_name: string;
}

function fromRaw(raw: RawActiveProcessInfo): ActiveProcessInfo {
  return {
    processId: raw.process_id,
    processName: raw.process_name,
    projectId: raw.project_id,
    projectName: raw.project_name,
  };
}

/**
 * Pushes the active Process (or `null` to clear it) to Rust, which
 * updates the system tray label and broadcasts the change to every
 * window. Fire-and-forget from the caller's point of view — a failure
 * here is logged on the backend and never needs frontend-visible error
 * handling (see docs/architecture.md).
 */
export async function syncActiveProcess(active: ActiveProcessInfo | null): Promise<void> {
  await invoke("sync_active_process", {
    input: active
      ? {
          process_id: active.processId,
          process_name: active.processName,
          project_id: active.projectId,
          project_name: active.projectName,
        }
      : null,
  });
}

/** Reads the current value directly — for a window (e.g. the widget)
 * that just opened and needs to know the active Process without waiting
 * for the next change event. */
export async function getActiveProcess(): Promise<ActiveProcessInfo | null> {
  const raw = await invoke<RawActiveProcessInfo | null>("get_active_process");
  return raw ? fromRaw(raw) : null;
}

/** Subscribes to live updates. Returns the unlisten function — callers
 * must call it on unmount to avoid leaking the listener. */
export async function onActiveProcessChanged(
  callback: (active: ActiveProcessInfo | null) => void,
): Promise<UnlistenFn> {
  return listen<RawActiveProcessInfo | null>(ACTIVE_PROCESS_CHANGED_EVENT, (event) => {
    callback(event.payload ? fromRaw(event.payload) : null);
  });
}
