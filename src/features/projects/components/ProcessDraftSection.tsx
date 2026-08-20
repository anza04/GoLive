import { useEffect, useState } from "react";
import { EmptyState } from "../../../components/ui/EmptyState";
import {
  generateProcessDraft,
  listProcessVersions,
  updateProcessVersionContent,
  type ProcessDraftStep,
  type ProcessVersion,
} from "../../../services/ai";
import { exportProcessVersionToDocx } from "../../../services/export";
import { getErrorMessage } from "../../../utils/errorMessage";
import { formatDate } from "../../../utils/formatDate";

interface ProcessDraftSectionProps {
  processId: string;
  /** Suggests a default export filename (TASK-020) — display only,
   * never sent back to the backend (the export command only ever takes
   * a version id; the .docx's own title comes from the Process's own
   * name, read fresh from the database at export time). */
  processName: string;
}

// TASK-017 gave this a "Generate" action and a plain read-only view.
// TASK-018 added real persistence (a `ProcessVersion` per generation,
// never overwritten). TASK-019 makes it a real, if deliberately simple,
// editor: the loaded version's summary and each step's title/
// description are plain editable text fields; saving updates that
// version's content *in place* (never creating a new version — only
// "Generate new version" does that); a dropdown switches between past
// versions. Explicitly out of scope (roadmap.md): rich formatting,
// reordering/adding/removing steps, collaborative editing.
type ListState = { state: "loading" } | { state: "ready" } | { state: "error"; message: string };

export function ProcessDraftSection({ processId, processName }: ProcessDraftSectionProps) {
  const [versions, setVersions] = useState<ProcessVersion[]>([]);
  const [listState, setListState] = useState<ListState>({ state: "loading" });
  const [selectedId, setSelectedId] = useState<string | null>(null);
  const [editSummary, setEditSummary] = useState("");
  const [editSteps, setEditSteps] = useState<ProcessDraftStep[]>([]);
  const [dirty, setDirty] = useState(false);
  const [saving, setSaving] = useState(false);
  const [saveError, setSaveError] = useState<string | null>(null);
  const [generating, setGenerating] = useState(false);
  const [generateError, setGenerateError] = useState<string | null>(null);
  const [exporting, setExporting] = useState(false);
  const [exportError, setExportError] = useState<string | null>(null);
  const [exportedPath, setExportedPath] = useState<string | null>(null);

  useEffect(() => {
    void refresh();
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [processId]);

  async function refresh() {
    setListState({ state: "loading" });
    try {
      const result = await listProcessVersions(processId);
      setVersions(result);
      setListState({ state: "ready" });
      if (result.length > 0) {
        loadIntoEditor(result[0]);
      } else {
        setSelectedId(null);
      }
    } catch (error) {
      setListState({ state: "error", message: getErrorMessage(error) });
    }
  }

  // Populates the edit buffers from `version` — used both when
  // switching versions and right after a fresh generation. Any unsaved
  // edits in the buffers are discarded without confirmation, the same
  // "no guard, just switch" convention every dialog's own Cancel button
  // in this app already follows.
  function loadIntoEditor(version: ProcessVersion) {
    setSelectedId(version.id);
    setEditSummary(version.content.summary);
    setEditSteps(version.content.steps);
    setDirty(false);
    setSaveError(null);
    setExportError(null);
    setExportedPath(null);
  }

  function handleVersionChange(id: string) {
    const version = versions.find((candidate) => candidate.id === id);
    if (version) loadIntoEditor(version);
  }

  function handleSummaryChange(value: string) {
    setEditSummary(value);
    setDirty(true);
    setExportedPath(null);
  }

  function handleStepChange(index: number, patch: Partial<Pick<ProcessDraftStep, "title" | "description">>) {
    setEditSteps((prev) => prev.map((step, i) => (i === index ? { ...step, ...patch } : step)));
    setDirty(true);
    setExportedPath(null);
  }

  async function handleGenerate() {
    setGenerating(true);
    setGenerateError(null);
    try {
      const version = await generateProcessDraft(processId);
      setVersions((prev) => [version, ...prev]);
      loadIntoEditor(version);
      setListState({ state: "ready" });
    } catch (error) {
      setGenerateError(getErrorMessage(error));
    } finally {
      setGenerating(false);
    }
  }

  async function handleSave() {
    if (!selectedId || saving) return;
    setSaving(true);
    setSaveError(null);
    try {
      const updated = await updateProcessVersionContent(selectedId, { summary: editSummary, steps: editSteps });
      setVersions((prev) => prev.map((version) => (version.id === updated.id ? updated : version)));
      setDirty(false);
    } catch (error) {
      setSaveError(getErrorMessage(error));
    } finally {
      setSaving(false);
    }
  }

  // Exports the currently selected version's *persisted* content — not
  // the live edit buffers — so a dirty, unsaved edit never silently
  // exports something the user hasn't actually saved yet (the Export
  // button is disabled while `dirty` is true; see the button below).
  async function handleExport() {
    if (!selectedId || exporting) return;
    setExporting(true);
    setExportError(null);
    setExportedPath(null);
    try {
      const path = await exportProcessVersionToDocx(selectedId, processName);
      if (path) setExportedPath(path);
      // `path` is null when the user cancelled the Save As dialog —
      // not an error, nothing to show.
    } catch (error) {
      setExportError(getErrorMessage(error));
    } finally {
      setExporting(false);
    }
  }

  if (listState.state === "loading") {
    return (
      <div className="process-draft-section">
        <h4 className="reserved-section__title">Process draft</h4>
        <p className="projects-status">Loading…</p>
      </div>
    );
  }

  if (listState.state === "error") {
    return (
      <div className="process-draft-section">
        <h4 className="reserved-section__title">Process draft</h4>
        <EmptyState
          title="Couldn't load the process draft"
          description={listState.message}
          action={
            <button type="button" className="button" onClick={() => void refresh()}>
              Retry
            </button>
          }
        />
      </div>
    );
  }

  const selectedVersion = versions.find((version) => version.id === selectedId) ?? null;

  return (
    <div className="process-draft-section">
      <div className="process-draft-section__header">
        <h4 className="reserved-section__title">Process draft</h4>
        <div className="process-draft-section__actions">
          {versions.length > 0 && (
            <select
              className="field__input"
              aria-label="Select a version"
              value={selectedId ?? ""}
              onChange={(event) => handleVersionChange(event.target.value)}
            >
              {versions.map((version) => (
                <option key={version.id} value={version.id}>
                  Generated {formatDate(version.createdAt)}
                </option>
              ))}
            </select>
          )}
          <button type="button" className="button" onClick={() => void handleGenerate()} disabled={generating}>
            {generating ? "Generating…" : versions.length > 0 ? "Generate new version" : "Generate"}
          </button>
        </div>
      </div>

      {versions.length === 0 && !generating && (
        <p className="process-draft-section__hint">
          Turns this process's captures into a structured, step-by-step draft using OpenAI.
        </p>
      )}

      {generateError && (
        <p className="dialog__error" role="alert">
          {generateError}
        </p>
      )}

      {generating && <p className="projects-status">Generating — this can take a moment…</p>}

      {selectedVersion && (
        <form
          className="process-draft-editor"
          onSubmit={(event) => {
            event.preventDefault();
            void handleSave();
          }}
        >
          <p className="process-draft__generated-at">
            Generated {formatDate(selectedVersion.createdAt)}
            {selectedVersion.updatedAt !== selectedVersion.createdAt &&
              ` · Edited ${formatDate(selectedVersion.updatedAt)}`}
          </p>

          <div className="field">
            <label className="field__label" htmlFor="process-draft-summary">
              Summary
            </label>
            <textarea
              id="process-draft-summary"
              className="field__textarea"
              rows={3}
              value={editSummary}
              onChange={(event) => handleSummaryChange(event.target.value)}
            />
          </div>

          <ol className="process-draft-editor__steps">
            {editSteps.map((step, index) => (
              <li key={index} className="process-draft-editor__step">
                <div className="field">
                  <label className="field__label" htmlFor={`process-draft-step-title-${index}`}>
                    Step {index + 1} title
                  </label>
                  <input
                    id={`process-draft-step-title-${index}`}
                    className="field__input"
                    value={step.title}
                    onChange={(event) => handleStepChange(index, { title: event.target.value })}
                  />
                </div>
                <div className="field">
                  <label className="field__label" htmlFor={`process-draft-step-description-${index}`}>
                    Description
                  </label>
                  <textarea
                    id={`process-draft-step-description-${index}`}
                    className="field__textarea"
                    rows={2}
                    value={step.description}
                    onChange={(event) => handleStepChange(index, { description: event.target.value })}
                  />
                </div>
                {step.captureIds.length > 0 && (
                  <p className="process-draft__step-refs">
                    Based on {step.captureIds.length} capture{step.captureIds.length === 1 ? "" : "s"}
                  </p>
                )}
              </li>
            ))}
          </ol>

          {saveError && (
            <p className="dialog__error" role="alert">
              {saveError}
            </p>
          )}

          <div className="dialog__footer">
            <button type="submit" className="button button--primary" disabled={!dirty || saving}>
              {saving ? "Saving…" : "Save"}
            </button>
            <button
              type="button"
              className="button"
              onClick={() => void handleExport()}
              disabled={dirty || exporting}
              title={dirty ? "Save your edits before exporting." : undefined}
            >
              {exporting ? "Exporting…" : "Export to Word"}
            </button>
          </div>

          {exportError && (
            <p className="dialog__error" role="alert">
              {exportError}
            </p>
          )}
          {exportedPath && <p className="process-draft__export-success">Exported to {exportedPath}</p>}
        </form>
      )}
    </div>
  );
}
