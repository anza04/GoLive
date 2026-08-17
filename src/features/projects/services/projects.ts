import { invoke } from "@tauri-apps/api/core";

export interface Project {
  id: string;
  name: string;
  description: string;
  /** Unix epoch milliseconds (UTC). Format for display with
   * `utils/formatDate` — never pre-formatted by the backend. */
  createdAt: number;
  updatedAt: number;
}

export interface CreateProjectInput {
  name: string;
  description?: string;
}

// The IPC wire shape: matches the Rust `Project` struct's field names
// exactly (snake_case), mapped to the camelCase `Project` above for use
// in React. Kept private to this file — everything else in the app only
// ever sees `Project`.
interface RawProject {
  id: string;
  name: string;
  description: string;
  created_at: number;
  updated_at: number;
}

function fromRaw(raw: RawProject): Project {
  return {
    id: raw.id,
    name: raw.name,
    description: raw.description,
    createdAt: raw.created_at,
    updatedAt: raw.updated_at,
  };
}

export async function createProject(input: CreateProjectInput): Promise<Project> {
  const raw = await invoke<RawProject>("create_project", { input });
  return fromRaw(raw);
}

export async function listProjects(): Promise<Project[]> {
  const raw = await invoke<RawProject[]>("list_projects");
  return raw.map(fromRaw);
}

export async function getProject(id: string): Promise<Project> {
  const raw = await invoke<RawProject>("get_project", { id });
  return fromRaw(raw);
}

export async function deleteProject(id: string): Promise<void> {
  await invoke("delete_project", { id });
}
