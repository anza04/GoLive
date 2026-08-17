import { invoke } from "@tauri-apps/api/core";

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
