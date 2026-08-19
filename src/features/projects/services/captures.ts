import { invoke } from "@tauri-apps/api/core";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import { formatDate } from "../../../utils/formatDate";

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

/**
 * Creates a zero-friction "marker" Capture (TASK-012): a `note`-type
 * Capture with an auto-generated title (a plain timestamp — using the
 * same `formatDate` every other display timestamp in the app uses, so
 * this reads the same way title text everywhere else does) and an empty
 * description. No dialog, no required input, no new backend command —
 * this is the exact same generic `createCapture` any hand-filled Note
 * capture goes through; only the frontend decides the title/description
 * on the caller's behalf (see roadmap.md TASK-012 and
 * docs/architecture.md, "Quick markers").
 */
export async function createQuickMarker(processId: string): Promise<Capture> {
  return createCapture({
    processId,
    captureType: "note",
    title: `Marker — ${formatDate(Date.now())}`,
    description: "",
  });
}

/** Input for `startRecordingCapture` — same shape as `CreateScreenshotInput`
 * (no `captureType` field; a recording operation always ends up
 * `type: "recording"`). */
export interface StartRecordingInput {
  processId: string;
  title: string;
  description?: string;
}

/**
 * "Is a recording in progress, and which one" — returned by
 * `startRecordingCapture`/`getRecordingStatus` and pushed by
 * `onRecordingStatusChanged` (TASK-014). Not a full `Capture`: no
 * metadata row exists until the recording is stopped (see
 * docs/architecture.md, "Screen recording capture engine and storage").
 * `startedAt` is a raw epoch-ms timestamp — elapsed time is computed
 * client-side (see `hooks/useElapsedSeconds`), the same
 * backend-sends-raw-timestamps rule every other date in the app follows.
 */
export interface RecordingStatus {
  id: string;
  processId: string;
  title: string;
  startedAt: number;
}

interface RawRecordingStatus {
  id: string;
  process_id: string;
  title: string;
  started_at: number;
}

function statusFromRaw(raw: RawRecordingStatus): RecordingStatus {
  return { id: raw.id, processId: raw.process_id, title: raw.title, startedAt: raw.started_at };
}

/**
 * Starts recording the primary display into `input.processId` — the
 * first half of the two-phase recording flow (TASK-013). Returns
 * immediately; the actual capture continues on the backend until
 * `stopRecordingCapture` is called. Rejects if a recording is already
 * in progress (system-wide — see docs/architecture.md, "one recording
 * at a time").
 */
export async function startRecordingCapture(input: StartRecordingInput): Promise<RecordingStatus> {
  const raw = await invoke<RawRecordingStatus>("start_recording_capture", {
    input: {
      process_id: input.processId,
      title: input.title,
      description: input.description,
    },
  });
  return statusFromRaw(raw);
}

/**
 * Stops the in-progress recording (blocks until the video file is fully
 * finalized on disk) and returns the resulting Recording Capture, with
 * its metadata row now created — the second half of the two-phase flow.
 * Rejects if no recording is currently in progress.
 */
export async function stopRecordingCapture(): Promise<Capture> {
  const raw = await invoke<RawCapture>("stop_recording_capture");
  return fromRaw(raw);
}

/** Reads the current recording status directly — for a window/section
 * that just mounted and needs to know "is a recording in progress"
 * without waiting for the next `onRecordingStatusChanged` push (see
 * `services/activeProcess.ts`'s `getActiveProcess` for the same
 * pattern). `null` means no recording is in progress. */
export async function getRecordingStatus(): Promise<RecordingStatus | null> {
  const raw = await invoke<RawRecordingStatus | null>("get_recording_status");
  return raw ? statusFromRaw(raw) : null;
}

const RECORDING_STATUS_CHANGED_EVENT = "recording-status-changed";

/** Subscribes to live recording start/stop updates — pushed to every
 * window (main window's Captures section, the floating widget) so
 * either one reflects a recording regardless of which one started it
 * (TASK-014). Returns the unlisten function — callers must call it on
 * unmount. */
export async function onRecordingStatusChanged(
  callback: (status: RecordingStatus | null) => void,
): Promise<UnlistenFn> {
  return listen<RawRecordingStatus | null>(RECORDING_STATUS_CHANGED_EVENT, (event) => {
    callback(event.payload ? statusFromRaw(event.payload) : null);
  });
}

/**
 * The default title used when starting a recording from a
 * button/hotkey with no title dialog (the Captures section's toolbar
 * control and the widget both use this) — same "auto-generated
 * timestamp title, reusing `formatDate`" convention `createQuickMarker`
 * (TASK-012) established.
 */
export function defaultRecordingTitle(): string {
  return `Recording — ${formatDate(Date.now())}`;
}

/**
 * Fetches a Recording Capture's MP4 bytes and returns a `blob:` object
 * URL a `<video>` can use directly — the Recording counterpart to
 * `getCaptureMediaUrl` (TASK-014 playback). Same "backend derives the
 * file from the id alone, caller owns the URL's lifetime and must
 * `URL.revokeObjectURL` it" contract.
 */
export async function getRecordingMediaUrl(captureId: string): Promise<string> {
  const bytes = await invoke<ArrayBuffer>("get_recording_media", { id: captureId });
  const blob = new Blob([bytes], { type: "video/mp4" });
  return URL.createObjectURL(blob);
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
