import { useEffect, useState } from "react";
import { getErrorMessage } from "../utils/errorMessage";
import { hideWidget } from "../services/widget";
import { getActiveProcess, onActiveProcessChanged, type ActiveProcessInfo } from "../services/activeProcess";
import {
  createQuickMarker,
  createScreenshotCapture,
  onScreenshotCaptured,
  type ScreenshotCaptureResult,
} from "../features/projects/services/captures";
import "./widget.css";

type Feedback = { kind: "ok" | "error"; message: string };

/**
 * The floating capture widget (TASK-011): a small always-on-top window,
 * separate from the main app — see docs/architecture.md, "Global hotkey
 * and floating capture widget". Its own React tree (Tauri windows are
 * separate webview runtimes with no shared JS memory), so it can't read
 * `stores/activeProcess.tsx`'s Context directly; instead it fetches the
 * active Process once on mount (`getActiveProcess`) and stays in sync
 * via `onActiveProcessChanged` — the same information the main window
 * pushed to Rust when the user selected/created/renamed a Process there.
 *
 * Deliberately minimal: no dialog, no title/description fields, no way
 * to switch the active Process from here — just "what's active" and two
 * buttons, matching the global hotkey's own no-dialog, instant-capture
 * behavior (see `hotkey.rs`). Screenshot and marker both share one
 * default title scheme rather than the widget asking for one, so using
 * either action always produces a predictable kind of Capture.
 *
 * "Add marker" (TASK-012) is the near-zero-friction way to flag a
 * moment during live work: one click creates a `note`-type Capture with
 * an auto-generated timestamp title and no description — no dialog, no
 * required input (see `services/captures.ts`'s `createQuickMarker`).
 */
export function Widget() {
  const [active, setActive] = useState<ActiveProcessInfo | null>(null);
  const [capturing, setCapturing] = useState(false);
  const [marking, setMarking] = useState(false);
  const [feedback, setFeedback] = useState<Feedback | null>(null);
  const busy = capturing || marking;

  useEffect(() => {
    let cancelled = false;

    void getActiveProcess().then((current) => {
      if (!cancelled) setActive(current);
    });

    const unlistenActive = onActiveProcessChanged((next) => setActive(next));
    const unlistenCaptured = onScreenshotCaptured((result) => setFeedback(feedbackFor(result)));

    return () => {
      cancelled = true;
      void unlistenActive.then((unlisten) => unlisten());
      void unlistenCaptured.then((unlisten) => unlisten());
    };
  }, []);

  async function handleCapture() {
    if (!active || busy) return;

    setCapturing(true);
    setFeedback(null);
    try {
      await createScreenshotCapture({ processId: active.processId, title: "Screenshot" });
      setFeedback({ kind: "ok", message: "Screenshot captured" });
    } catch (err) {
      setFeedback({ kind: "error", message: getErrorMessage(err) });
    } finally {
      setCapturing(false);
    }
  }

  async function handleMarker() {
    if (!active || busy) return;

    setMarking(true);
    setFeedback(null);
    try {
      await createQuickMarker(active.processId);
      setFeedback({ kind: "ok", message: "Marker added" });
    } catch (err) {
      setFeedback({ kind: "error", message: getErrorMessage(err) });
    } finally {
      setMarking(false);
    }
  }

  return (
    <div className="widget">
      <div className="widget__header" data-tauri-drag-region>
        <span className="widget__title" data-tauri-drag-region>
          GoLive
        </span>
        <button
          type="button"
          className="widget__hide"
          onClick={() => void hideWidget()}
          aria-label="Hide widget"
          title="Hide widget"
        >
          ×
        </button>
      </div>

      <div className="widget__body">
        {active ? (
          <div className="widget__active">
            <p className="widget__process">{active.processName}</p>
            <p className="widget__project">{active.projectName}</p>
          </div>
        ) : (
          <p className="widget__empty">No active process — open GoLive and select one.</p>
        )}

        <div className="widget__actions">
          <button
            type="button"
            className="button button--primary widget__capture"
            onClick={() => void handleCapture()}
            disabled={!active || busy}
          >
            {capturing ? "Capturing…" : "Capture screenshot"}
          </button>

          <button
            type="button"
            className="button widget__marker"
            onClick={() => void handleMarker()}
            disabled={!active || busy}
          >
            {marking ? "Adding…" : "Add marker"}
          </button>
        </div>

        {feedback && (
          <p className={`widget__feedback widget__feedback--${feedback.kind}`} role="status">
            {feedback.message}
          </p>
        )}
      </div>
    </div>
  );
}

function feedbackFor(result: ScreenshotCaptureResult): Feedback {
  switch (result.status) {
    case "ok":
      return { kind: "ok", message: "Screenshot captured" };
    case "error":
      return { kind: "error", message: result.message };
    case "no_active_process":
      return { kind: "error", message: "No active process selected" };
  }
}
