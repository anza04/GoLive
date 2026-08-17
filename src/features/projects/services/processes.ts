import { invoke } from "@tauri-apps/api/core";

export type ProcessStatus = "draft" | "in_progress" | "completed";

export interface Process {
  id: string;
  projectId: string;
  name: string;
  description: string;
  status: ProcessStatus;
  /** Unix epoch milliseconds (UTC). Format for display with
   * `utils/formatDate` — never pre-formatted by the backend. */
  createdAt: number;
  updatedAt: number;
}

export interface CreateProcessInput {
  projectId: string;
  name: string;
  description?: string;
}

export interface UpdateProcessInput {
  id: string;
  name: string;
  description?: string;
  status: ProcessStatus;
}

// The IPC wire shape: matches the Rust `Process` struct's field names
// exactly (snake_case). `status` needs no translation — the backend
// already serializes it as one of the three stable lowercase strings
// that make up the `ProcessStatus` union above.
interface RawProcess {
  id: string;
  project_id: string;
  name: string;
  description: string;
  status: ProcessStatus;
  created_at: number;
  updated_at: number;
}

function fromRaw(raw: RawProcess): Process {
  return {
    id: raw.id,
    projectId: raw.project_id,
    name: raw.name,
    description: raw.description,
    status: raw.status,
    createdAt: raw.created_at,
    updatedAt: raw.updated_at,
  };
}

export async function createProcess(input: CreateProcessInput): Promise<Process> {
  const raw = await invoke<RawProcess>("create_process", {
    input: {
      project_id: input.projectId,
      name: input.name,
      description: input.description,
    },
  });
  return fromRaw(raw);
}

export async function listProcesses(projectId: string): Promise<Process[]> {
  const raw = await invoke<RawProcess[]>("list_processes", {
    input: { project_id: projectId },
  });
  return raw.map(fromRaw);
}

export async function getProcess(id: string): Promise<Process> {
  const raw = await invoke<RawProcess>("get_process", { id });
  return fromRaw(raw);
}

export async function updateProcess(input: UpdateProcessInput): Promise<Process> {
  const raw = await invoke<RawProcess>("update_process", { input });
  return fromRaw(raw);
}

export async function deleteProcess(id: string): Promise<void> {
  await invoke("delete_process", { id });
}
