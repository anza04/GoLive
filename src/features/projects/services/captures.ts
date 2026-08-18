import { invoke } from "@tauri-apps/api/core";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";

export type CaptureType = "screenshot" | "recording" | "note";

export interface Capture {
  id: string;
  processId: string;
  type: CaptureType;
  title: string;
  description: string;
  /** Unix epoch milliseconds (UTC). Format for display with
   * `utils/formatDate` — never pre-formatted by the backend. */
  createdAt: number;
  updatedAt: number;
}

export interface CreateCaptureInput {
  processId: string;
  captureType: CaptureType;
  title: string;
  description?: string;
}

export interface UpdateCaptureInput {
  id: string;
  captureType: CaptureType;
  title: string;
  description?: string;
}

/** Input for `createScreenshotCapture` — deliberately has no `captureType`
 * field at all. A screenshot operation always produces a
 * `type: "screenshot"` Capture; there is no way to ask it for anything
 * else (see docs/architecture.md, "Screenshot creation is
 * transactional"). */
export interface CreateScreenshotInput {
  processId: string;
  title: string;
  description?: string;
}

// The IPC wire shape: matches the Rust `Capture` struct's field names
// exactly (snake_case). `type` needs no translation — the backend
// already serializes it as one of the three stable lowercase strings
// that make up the `CaptureType` union above.
interface RawCapture {
  id: string;
  process_id: string;
  type: CaptureType;
  title: string;
  description: string;
  created_at: number;
  updated_at: number;
}

function fromRaw(raw: RawCapture): Capture {
  return {
    id: raw.id,
    processId: raw.process_id,
    type: raw.type,
    title: raw.title,
    description: raw.description,
    createdAt: raw.created_at,
    updatedAt: raw.updated_at,
  };
}

export async function createCapture(input: CreateCaptureInput): Promise<Capture> {
  const raw = await invoke<RawCapture>("create_capture", {
    input: {
      process_id: input.processId,
      capture_type: input.captureType,
      title: input.title,
      description: input.description,
    },
  });
  return fromRaw(raw);
}

/**
 * Captures the primary Windows display, stores it, and creates the
 * Capture record — the real-media counterpart to `createCapture`. Always
 * produces `type: "screenshot"`; there is no `captureType` parameter to
 * pass (see `CreateScreenshotInput`). The frontend never sees a
 * filesystem path — the backend determines storage location, filename,
 * id, and timestamps entirely on its own.
 */
export async function createScreenshotCapture(input: CreateScreenshotInput): Promise<Capture> {
  const raw = await invoke<RawCapture>("create_screenshot_capture", {
    input: {
      process_id: input.processId,
      title: input.title,
      description: input.description,
    },
  });
  return fromRaw(raw);
}

export async function listCaptures(processId: string): Promise<Capture[]> {
  const raw = await invoke<RawCapture[]>("list_captures", {
    input: { process_id: processId },
  });
  return raw.map(fromRaw);
}

export async function getCapture(id: string): Promise<Capture> {
  const raw = await invoke<RawCapture>("get_capture", { id });
  return fromRaw(raw);
}

/**
 * Fetches a screenshot Capture's PNG bytes and returns a `blob:` object
 * URL an `<img>` can use directly. The backend derives the file from
 * `captureId` alone — this never sends or receives a filesystem path
 * (see docs/architecture.md, "Safe media access"). Rejects with the
 * usual `AppError` shape (via `getErrorMessage`/`isNotFoundError`) if the
 * capture has no media.
 *
 * The caller owns the returned URL's lifetime: call
 * `URL.revokeObjectURL(url)` once it's no longer displayed (e.g. on
 * unmount or before fetching a different capture's media), or the blob
 * leaks for the lifetime of the page.
 */
export async function getCaptureMediaUrl(captureId: string): Promise<string> {
  const bytes = await invoke<ArrayBuffer>("get_capture_media", { id: captureId });
  const blob = new Blob([bytes], { type: "image/png" });
  return URL.createObjectURL(blob);
}

export async function updateCapture(input: UpdateCaptureInput): Promise<Capture> {
  const raw = await invoke<RawCapture>("update_capture", {
    input: {
      id: input.id,
      capture_type: input.captureType,
      title: input.title,
      description: input.description,
    },
  });
  return fromRaw(raw);
}

export async function deleteCapture(id: string): Promise<void> {
  await invoke("delete_capture", { id });
}

/**
 * The outcome of a screenshot capture triggered *outside* any dialog's
 * own submit flow — i.e. the global hotkey (TASK-011), which has no
 * requesting window/button to report failure through directly. Mirrors
 * the Rust `hotkey::CaptureResult` tagged enum exactly (`status` tag);
 * see `hotkey.rs`'s own serialization test for the wire shape this
 * relies on.
 */
export type ScreenshotCaptureResult =
  | { status: "ok" }
  | { status: "error"; message: string }
  | { status: "no_active_process" };

const SCREENSHOT_CAPTURED_EVENT = "screenshot-captured";

/** Subscribes to hotkey-triggered screenshot-capture results (see
 * `hotkey.rs`). Returns the unlisten function — callers must call it on
 * unmount. Only the floating widget listens for this today; the main
 * window's own Create Capture dialog already gets its result as a
 * normal `createScreenshotCapture` return value/thrown error. */
export async function onScreenshotCaptured(
  callback: (result: ScreenshotCaptureResult) => void,
): Promise<UnlistenFn> {
  return listen<ScreenshotCaptureResult>(SCREENSHOT_CAPTURED_EVENT, (event) => {
    callback(event.payload);
  });
}
