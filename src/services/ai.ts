import { invoke } from "@tauri-apps/api/core";

/**
 * TASK-017/018: generates a structured process draft from a Process's
 * Captures via OpenAI, persisted as an immutable `ProcessVersion` —
 * regenerating creates a new version, never overwriting a previous one
 * (see docs/architecture.md §5/§32).
 */

export interface ProcessDraftStep {
  title: string;
  description: string;
  /** Which Capture(s) this step is based on — may be empty (a step the
   * model synthesized rather than tying to one specific capture). */
  captureIds: string[];
}

export interface ProcessDraft {
  summary: string;
  steps: ProcessDraftStep[];
}

/** One persisted, immutable generation result (TASK-018). */
export interface ProcessVersion {
  id: string;
  processId: string;
  content: ProcessDraft;
  /** Unix epoch milliseconds (UTC). Format for display with
   * `utils/formatDate` — never pre-formatted by the backend. */
  createdAt: number;
}

interface RawProcessDraftStep {
  title: string;
  description: string;
  capture_ids: string[];
}

interface RawProcessDraft {
  summary: string;
  steps: RawProcessDraftStep[];
}

interface RawProcessVersion {
  id: string;
  process_id: string;
  content: RawProcessDraft;
  created_at: number;
}

function draftFromRaw(raw: RawProcessDraft): ProcessDraft {
  return {
    summary: raw.summary,
    steps: raw.steps.map((step) => ({
      title: step.title,
      description: step.description,
      captureIds: step.capture_ids,
    })),
  };
}

function versionFromRaw(raw: RawProcessVersion): ProcessVersion {
  return {
    id: raw.id,
    processId: raw.process_id,
    content: draftFromRaw(raw.content),
    createdAt: raw.created_at,
  };
}

/**
 * Rejects with the usual `AppError` shape (via `getErrorMessage`) if:
 * no API key is saved, the process has no captures, OpenAI rejects the
 * key or has no quota, or the network call otherwise fails. Can take a
 * while (a real OpenAI call, possibly with several images) — callers
 * should show a busy state, not assume this resolves quickly. Returns
 * the newly created, persisted version — not a bare, ephemeral draft.
 */
export async function generateProcessDraft(processId: string): Promise<ProcessVersion> {
  const raw = await invoke<RawProcessVersion>("generate_process_draft", {
    input: { process_id: processId },
  });
  return versionFromRaw(raw);
}

/** Every version of `processId`, newest first. */
export async function listProcessVersions(processId: string): Promise<ProcessVersion[]> {
  const raw = await invoke<RawProcessVersion[]>("list_process_versions", {
    input: { process_id: processId },
  });
  return raw.map(versionFromRaw);
}

/** The newest version for `processId`, or `null` if it's never been
 * generated — lets a fresh page load show the last real generation
 * instead of starting blank. */
export async function getLatestProcessVersion(processId: string): Promise<ProcessVersion | null> {
  const raw = await invoke<RawProcessVersion | null>("get_latest_process_version", {
    input: { process_id: processId },
  });
  return raw ? versionFromRaw(raw) : null;
}
