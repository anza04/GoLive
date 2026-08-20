import { useEffect, useState } from "react";
import { generateProcessDraft, getLatestProcessVersion, type ProcessVersion } from "../../../services/ai";
import { getErrorMessage } from "../../../utils/errorMessage";
import { formatDate } from "../../../utils/formatDate";

interface ProcessDraftSectionProps {
  processId: string;
}

// TASK-017: a "Generate" action and a plain read-only view of whatever
// came back — deliberately not the polished editor (TASK-019). TASK-018
// added real persistence: each generation is saved as an immutable
// `ProcessVersion`, never overwriting a previous one, so on mount this
// loads and shows the latest saved version instead of always starting
// blank — a Process's most recent draft survives leaving and returning
// to the page, the same way its Captures already do.
type DraftState =
  | { state: "loading" }
  | { state: "empty" }
  | { state: "generating" }
  | { state: "ready"; version: ProcessVersion }
  | { state: "error"; message: string };

export function ProcessDraftSection({ processId }: ProcessDraftSectionProps) {
  const [draftState, setDraftState] = useState<DraftState>({ state: "loading" });

  useEffect(() => {
    let cancelled = false;
    setDraftState({ state: "loading" });
    getLatestProcessVersion(processId)
      .then((version) => {
        if (cancelled) return;
        setDraftState(version ? { state: "ready", version } : { state: "empty" });
      })
      .catch((error) => {
        if (cancelled) return;
        setDraftState({ state: "error", message: getErrorMessage(error) });
      });
    return () => {
      cancelled = true;
    };
  }, [processId]);

  async function handleGenerate() {
    setDraftState({ state: "generating" });
    try {
      const version = await generateProcessDraft(processId);
      setDraftState({ state: "ready", version });
    } catch (error) {
      setDraftState({ state: "error", message: getErrorMessage(error) });
    }
  }

  const busy = draftState.state === "generating";
  const hasVersion = draftState.state === "ready";

  return (
    <div className="process-draft-section">
      <div className="process-draft-section__header">
        <h4 className="reserved-section__title">AI analysis</h4>
        <button type="button" className="button" onClick={() => void handleGenerate()} disabled={busy || draftState.state === "loading"}>
          {busy ? "Generating…" : hasVersion ? "Regenerate" : "Generate"}
        </button>
      </div>

      {draftState.state === "loading" && <p className="projects-status">Loading…</p>}

      {draftState.state === "empty" && (
        <p className="process-draft-section__hint">
          Turns this process's captures into a structured, step-by-step draft using OpenAI.
        </p>
      )}

      {draftState.state === "generating" && <p className="projects-status">Generating — this can take a moment…</p>}

      {draftState.state === "error" && (
        <p className="dialog__error" role="alert">
          {draftState.message}
        </p>
      )}

      {draftState.state === "ready" && (
        <div className="process-draft">
          <p className="process-draft__generated-at">Generated {formatDate(draftState.version.createdAt)}</p>
          <p className="process-draft__summary">{draftState.version.content.summary}</p>
          <ol className="process-draft__steps">
            {draftState.version.content.steps.map((step, index) => (
              <li key={index} className="process-draft__step">
                <h5 className="process-draft__step-title">{step.title}</h5>
                <p className="process-draft__step-description">{step.description}</p>
                {step.captureIds.length > 0 && (
                  <p className="process-draft__step-refs">
                    Based on {step.captureIds.length} capture{step.captureIds.length === 1 ? "" : "s"}
                  </p>
                )}
              </li>
            ))}
          </ol>
        </div>
      )}
    </div>
  );
}
