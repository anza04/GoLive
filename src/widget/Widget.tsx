import { useEffect, useState } from "react";
import { getErrorMessage } from "../utils/errorMessage";
import { formatElapsed } from "../utils/formatElapsed";
import { useElapsedSeconds } from "../hooks/useElapsedSeconds";
import { hideWidget, setWidgetExpanded } from "../services/widget";
import { getActiveProcess, onActiveProcessChanged, type ActiveProcessInfo } from "../services/activeProcess";
import {
  createQuickMarker,
  createScreenshotCapture,
  defaultRecordingTitle,
  getRecordingStatus,
  onRecordingStatusChanged,
  onScreenshotCaptured,
  startRecordingCapture,
  stopRecordingCapture,
  type RecordingStatus,
  type ScreenshotCaptureResult,
} from "../features/projects/services/captures";
import "./widget.css";

type Feedback = { kind: "ok" | "error"; message: string };

/**
 * The floating capture widget (TASK-011, extended TASK-012/TASK-013/
 * TASK-014): a small always-on-top window, separate from the main app —
 * see docs/architecture.md, "Global hotkey and floating capture widget".
 * Its own React tree (Tauri windows are separate webview runtimes with
 * no shared JS memory), so it can't read `stores/activeProcess.tsx`'s
 * Context directly; instead it fetches the active Process once on mount
 * (`getActiveProcess`) and stays in sync via `onActiveProcessChanged` —
 * the same information the main window pushed to Rust when the user
 * selected/created/renamed a Process there. Recording status is synced
 * the same way (`getRecordingStatus`/`onRecordingStatusChanged`) since
 * recording is system-wide, not owned by any one window (see
 * `recording.rs`).
 *
 * Has two in-window states (TASK-014 bugfix): collapsed to a small
 * draggable **dot** (the resting default — see `widget.css`,
 * `.widget--dot`) or **expanded** to the full panel with its buttons.
 * Clicking the dot expands it; the panel's "–" button collapses back.
 * The window itself is resized to match (`setWidgetExpanded`,
 * `commands::widget::set_widget_expanded`) rather than keeping one
 * fixed-size window and hiding content with CSS, so the dot is a
 * genuinely small window, not a small shape inside a large invisible
 * one. The whole dot (and the panel's header) carries
 * `data-tauri-drag-region` so the widget can be dragged anywhere on
 * screen in either state — see the widget's capability file for the
 * `core:window:allow-start-dragging` permission this requires.
 *
 * Deliberately minimal: no dialog, no title/description fields, no way
 * to switch the active Process from here — just "what's active" and a
 * handful of buttons, matching the global hotkey's own no-dialog,
 * instant-capture behavior (see `hotkey.rs`). Screenshot, marker, and
 * recording each use an auto-generated default title rather than the
 * widget asking for one.
 */
export function Widget() {
  const [expanded, setExpanded] = useState(false);
  const [active, setActive] = useState<ActiveProcessInfo | null>(null);
  const [capturing, setCapturing] = useState(false);
  const [marking, setMarking] = useState(false);
  const [recordingStatus, setRecordingStatus] = useState<RecordingStatus | null>(null);
  const [recordingBusy, setRecordingBusy] = useState(false);
  // Opt-in microphone toggle (TASK-015) — same "plain local state, not
  // persisted, read once when Start is clicked" treatment as
  // `CapturesSection`'s own copy of this control.
  const [includeAudio, setIncludeAudio] = useState(false);
  const [feedback, setFeedback] = useState<Feedback | null>(null);
  const busy = capturing || marking;

  useEffect(() => {
    let cancelled = false;

    void getActiveProcess().then((current) => {
      if (!cancelled) setActive(current);
    });
    void getRecordingStatus().then((status) => {
      if (!cancelled) setRecordingStatus(status);
    });

    const unlistenActive = onActiveProcessChanged((next) => setActive(next));
    const unlistenCaptured = onScreenshotCaptured((result) => setFeedback(feedbackFor(result)));
    const unlistenRecording = onRecordingStatusChanged((status) => setRecordingStatus(status));

    return () => {
      cancelled = true;
      void unlistenActive.then((unlisten) => unlisten());
      void unlistenCaptured.then((unlisten) => unlisten());
      void unlistenRecording.then((unlisten) => unlisten());
    };
  }, []);

  function handleToggleExpanded(next: boolean) {
    setExpanded(next);
    void setWidgetExpanded(next);
  }

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

  async function handleStartRecording() {
    if (!active || recordingBusy) return;

    setRecordingBusy(true);
    setFeedback(null);
    try {
      const status = await startRecordingCapture({
        processId: active.processId,
        title: defaultRecordingTitle(),
        includeAudio,
      });
      setRecordingStatus(status);
    } catch (err) {
      setFeedback({ kind: "error", message: getErrorMessage(err) });
    } finally {
      setRecordingBusy(false);
    }
  }

  async function handleStopRecording() {
    if (recordingBusy) return;

    setRecordingBusy(true);
    setFeedback(null);
    try {
      await stopRecordingCapture();
      setRecordingStatus(null);
      setFeedback({ kind: "ok", message: "Recording saved" });
    } catch (err) {
      setFeedback({ kind: "error", message: getErrorMessage(err) });
    } finally {
      setRecordingBusy(false);
    }
  }

  const activeRecording =
    recordingStatus && recordingStatus.processId === active?.processId ? recordingStatus : null;
  const isRecordingElsewhere = recordingStatus !== null && activeRecording === null;

  if (!expanded) {
    return (
      <button
        type="button"
        className="widget-dot"
        data-tauri-drag-region
        onClick={() => handleToggleExpanded(true)}
        aria-label="Open GoLive capture panel"
        title="GoLive — click to open"
      >
        <span className="widget-dot__label" data-tauri-drag-region>
          G
        </span>
        {recordingStatus && <span className="widget-dot__recording" aria-label="Recording in progress" />}
      </button>
    );
  }

  return (
    <div className="widget">
      <div className="widget__header" data-tauri-drag-region>
        <span className="widget__title" data-tauri-drag-region>
          GoLive
        </span>
        <div className="widget__header-actions">
          <button
            type="button"
            className="widget__collapse"
            onClick={() => handleToggleExpanded(false)}
            aria-label="Collapse to dot"
            title="Collapse to dot"
          >
            –
          </button>
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

          {activeRecording ? (
            <div className="widget__recording-row">
              <span className="widget__recording-elapsed" role="status">
                ● <ElapsedTime startedAt={activeRecording.startedAt} />
              </span>
              <button
                type="button"
                className="button button--danger widget__record-stop"
                onClick={() => void handleStopRecording()}
                disabled={recordingBusy}
              >
                {recordingBusy ? "Stopping…" : "Stop recording"}
              </button>
            </div>
          ) : (
            <div className="widget__record-group">
              <label className="widget__audio-toggle">
                <input
                  type="checkbox"
                  checked={includeAudio}
                  onChange={(event) => setIncludeAudio(event.target.checked)}
                  disabled={recordingBusy || isRecordingElsewhere}
                />
                Include microphone audio
              </label>
              <button
                type="button"
                className="button widget__record"
                onClick={() => void handleStartRecording()}
                disabled={!active || recordingBusy || isRecordingElsewhere}
                title={isRecordingElsewhere ? "A recording is already in progress for another process." : undefined}
              >
                {recordingBusy ? "Starting…" : "Start recording"}
              </button>
            </div>
          )}
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

function ElapsedTime({ startedAt }: { startedAt: number }) {
  const seconds = useElapsedSeconds(startedAt);
  return <>{formatElapsed(seconds)}</>;
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
