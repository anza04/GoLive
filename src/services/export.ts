import { invoke } from "@tauri-apps/api/core";
import { save } from "@tauri-apps/plugin-dialog";

/**
 * TASK-020: exports a `ProcessVersion`'s content to a real Word (.docx)
 * functional specification, at a location the user picks themselves via
 * a native Save As dialog (`@tauri-apps/plugin-dialog`'s `save()`) — the
 * first place in GoLive a user chooses a filesystem destination for
 * anything. See docs/architecture.md, "Word export".
 *
 * Follow-up: the same export, as a LaTeX source bundle instead of a
 * `.docx` — a second format sharing this module's `save()`-dialog
 * pattern, not a second UI concept. LaTeX has no way to embed an image
 * inline in a single self-contained file the way `.docx` does, so this
 * one produces a `.zip` (`document.tex` + `images/` + a short
 * `README.txt`) rather than one document file — still exactly one file
 * the user picks through the same kind of dialog.
 */

/** Strips characters Windows filenames can't contain, so a process name
 * is always safe to suggest as a default export filename. Collapses to
 * "Process" if nothing usable is left (e.g. a name that was only
 * forbidden characters). */
function suggestedFileName(processName: string, extension: string): string {
  const cleaned = processName.replace(/[\\/:*?"<>|]/g, "").trim();
  return `${cleaned || "Process"}.${extension}`;
}

/**
 * Opens a native Save As dialog defaulting to `${processName}.docx`,
 * then — if the user didn't cancel — exports `versionId`'s content
 * there. Returns the chosen path on success, or `null` if the user
 * cancelled the dialog (not an error). Rejects with the usual `AppError`
 * shape (via `getErrorMessage`) if the export itself fails.
 */
export async function exportProcessVersionToDocx(versionId: string, processName: string): Promise<string | null> {
  const targetPath = await save({
    defaultPath: suggestedFileName(processName, "docx"),
    filters: [{ name: "Word Document", extensions: ["docx"] }],
  });
  if (!targetPath) {
    return null;
  }

  await invoke("export_process_version_to_docx", {
    input: { version_id: versionId, target_path: targetPath },
  });
  return targetPath;
}

/**
 * The LaTeX counterpart to `exportProcessVersionToDocx` — same shape
 * (native Save As dialog, `null` on cancel, the chosen path on
 * success), targeting a `.zip` source bundle instead of a `.docx`.
 */
export async function exportProcessVersionToLatex(versionId: string, processName: string): Promise<string | null> {
  const targetPath = await save({
    defaultPath: suggestedFileName(processName, "zip"),
    filters: [{ name: "LaTeX Source Bundle", extensions: ["zip"] }],
  });
  if (!targetPath) {
    return null;
  }

  await invoke("export_process_version_to_latex", {
    input: { version_id: versionId, target_path: targetPath },
  });
  return targetPath;
}
