import { useEffect, useState } from "react";
import { EmptyState } from "../../../components/ui/EmptyState";
import { getErrorMessage } from "../../../utils/errorMessage";
import { formatElapsed } from "../../../utils/formatElapsed";
import { useElapsedSeconds } from "../../../hooks/useElapsedSeconds";
import { CaptureList } from "./CaptureList";
import { CaptureDetail } from "./CaptureDetail";
import { CreateCaptureDialog } from "./CreateCaptureDialog";
import { DeleteCaptureDialog } from "./DeleteCaptureDialog";
import {
  defaultRecordingTitle,
  getRecordingStatus,
  listCaptures,
  onRecordingStatusChanged,
  startRecordingCapture,
  stopRecordingCapture,
  type Capture,
  type RecordingStatus,
} from "../services/captures";

interface CapturesSectionProps {
  processId: string;
}

// Same shape as ProcessesView's own list+detail state (see
// docs/architecture.md, "State management") — Captures is nested one
// level deeper inside ProcessDetail, but deliberately kept just as
// simple: a plain `selectedId`, no nested workspace of its own.
type ListState = { state: "loading" } | { state: "ready" } | { state: "error"; message: string };

export function CapturesSection({ processId }: CapturesSectionProps) {
  const [captures, setCaptures] = useState<Capture[]>([]);
  const [listState, setListState] = useState<ListState>({ state: "loading" });
  const [selectedId, setSelectedId] = useState<string | null>(null);
  const [createOpen, setCreateOpen] = useState(false);
  const [deleteTarget, setDeleteTarget] = useState<Capture | null>(null);

  // Recording is system-wide, not scoped to this Process (see
  // docs/architecture.md, "one recording at a time") — this status is
  // fetched once and kept live via `onRecordingStatusChanged` for the
  // component's whole lifetime, independent of which Process is
  // currently selected (TASK-014).
  const [recordingStatus, setRecordingStatus] = useState<RecordingStatus | null>(null);
  const [recordingBusy, setRecordingBusy] = useState(false);
  const [recordingError, setRecordingError] = useState<string | null>(null);

  useEffect(() => {
    void refresh();
    setSelectedId(null);
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [processId]);

  useEffect(() => {
    let cancelled = false;

    void getRecordingStatus().then((status) => {
      if (!cancelled) setRecordingStatus(status);
    });

    const unlisten = onRecordingStatusChanged((status) => setRecordingStatus(status));

    return () => {
      cancelled = true;
      void unlisten.then((fn) => fn());
    };
  }, []);

  async function refresh() {
    setListState({ state: "loading" });
    try {
      const result = await listCaptures(processId);
      setCaptures(result);
      setListState({ state: "ready" });
    } catch (error) {
      setListState({ state: "error", message: getErrorMessage(error) });
    }
  }

  function handleCreated(capture: Capture) {
    setCaptures((prev) => [capture, ...prev]);
    setSelectedId(capture.id);
    setCreateOpen(false);
  }

  function handleUpdated(updated: Capture) {
    // Matches the backend's `updated_at DESC` order without a re-fetch.
    setCaptures((prev) => [updated, ...prev.filter((capture) => capture.id !== updated.id)]);
  }

  function handleGone(id: string) {
    setCaptures((prev) => prev.filter((capture) => capture.id !== id));
    setSelectedId((current) => (current === id ? null : current));
    setDeleteTarget(null);
  }

  async function handleStartRecording() {
    if (recordingBusy) return;
    setRecordingBusy(true);
    setRecordingError(null);
    try {
      const status = await startRecordingCapture({ processId, title: defaultRecordingTitle() });
      setRecordingStatus(status);
    } catch (err) {
      setRecordingError(getErrorMessage(err));
    } finally {
      setRecordingBusy(false);
    }
  }

  async function handleStopRecording() {
    if (recordingBusy) return;
    setRecordingBusy(true);
    setRecordingError(null);
    try {
      const capture = await stopRecordingCapture();
      setRecordingStatus(null);
      // Only add it to *this* section's list if it belongs to the
      // Process currently shown — a recording started (and stopped)
      // while a different Process was selected shouldn't appear here.
      if (capture.processId === processId) {
        handleCreated(capture);
      }
    } catch (err) {
      setRecordingError(getErrorMessage(err));
    } finally {
      setRecordingBusy(false);
    }
  }

  const newCaptureButton = (
    <button type="button" className="button button--primary" onClick={() => setCreateOpen(true)}>
      + New capture
    </button>
  );

  const recordingControl = (
    <RecordingControl
      processId={processId}
      status={recordingStatus}
      busy={recordingBusy}
      error={recordingError}
      onStart={() => void handleStartRecording()}
      onStop={() => void handleStopRecording()}
    />
  );

  if (listState.state === "loading") {
    return (
      <div className="captures-section">
        <h4 className="reserved-section__title">Captures</h4>
        <p className="projects-status">Loading captures…</p>
      </div>
    );
  }

  if (listState.state === "error") {
    return (
      <div className="captures-section">
        <h4 className="reserved-section__title">Captures</h4>
        <EmptyState
          title="Couldn't load captures"
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

  if (captures.length === 0) {
    return (
      <div className="captures-section">
        <h4 className="reserved-section__title">Captures</h4>
        <div className="projects-toolbar">
          {newCaptureButton}
          {recordingControl}
        </div>
        <EmptyState
          title="No captures yet"
          description="Create a capture to start collecting evidence for this process."
        />
        {createOpen && (
          <CreateCaptureDialog
            processId={processId}
            onClose={() => setCreateOpen(false)}
            onCreated={handleCreated}
          />
        )}
      </div>
    );
  }

  const selectedCapture = captures.find((capture) => capture.id === selectedId) ?? null;

  return (
    <div className="captures-section">
      <h4 className="reserved-section__title">Captures</h4>
      <div className="captures-layout">
        <div className="captures-list-pane">
          <div className="projects-toolbar">
            {newCaptureButton}
            {recordingControl}
          </div>
          <CaptureList captures={captures} selectedId={selectedId} onSelect={setSelectedId} />
        </div>

        <div className="captures-detail-pane">
          {selectedCapture ? (
            <CaptureDetail
              capture={selectedCapture}
              onUpdated={handleUpdated}
              onDeleteRequested={() => setDeleteTarget(selectedCapture)}
              onGone={() => handleGone(selectedCapture.id)}
            />
          ) : (
            <EmptyState
              title="No capture selected"
              description="Select a capture from the list to see its details."
            />
          )}
        </div>
      </div>

      {createOpen && (
        <CreateCaptureDialog
          processId={processId}
          onClose={() => setCreateOpen(false)}
          onCreated={handleCreated}
        />
      )}
      {deleteTarget && (
        <DeleteCaptureDialog
          capture={deleteTarget}
          onClose={() => setDeleteTarget(null)}
          onDeleted={handleGone}
        />
      )}
    </div>
  );
}

interface RecordingControlProps {
  processId: string;
  status: RecordingStatus | null;
  busy: boolean;
  error: string | null;
  onStart: () => void;
  onStop: () => void;
}

/**
 * The Captures section's Start/Stop recording control (TASK-014) — a
 * toolbar-level control, not nested inside `CreateCaptureDialog`, so a
 * recording can run while the user keeps browsing/selecting other
 * captures instead of being stuck behind a blocking modal for however
 * long the recording lasts (see DECISIONS.md). Recording is system-wide
 * (`status` reflects it regardless of which Process started it — see
 * `recording::RecordingState`), so this also handles "a recording is
 * running, but for a *different* Process" by disabling Start rather
 * than pretending nothing is happening.
 */
function RecordingControl({ processId, status, busy, error, onStart, onStop }: RecordingControlProps) {
  const isRecordingHere = status?.processId === processId;
  const isRecordingElsewhere = status !== null && !isRecordingHere;
  const elapsedSeconds = useElapsedSeconds(isRecordingHere ? status.startedAt : null);

  return (
    <div className="recording-control">
      {isRecordingHere ? (
        <>
          <span className="recording-control__indicator" role="status">
            ● Recording {formatElapsed(elapsedSeconds)}
          </span>
          <button type="button" className="button button--danger" onClick={onStop} disabled={busy}>
            {busy ? "Stopping…" : "Stop recording"}
          </button>
        </>
      ) : (
        <button
          type="button"
          className="button"
          onClick={onStart}
          disabled={busy || isRecordingElsewhere}
          title={isRecordingElsewhere ? "A recording is already in progress for another process." : undefined}
        >
          {busy ? "Starting…" : "Start recording"}
        </button>
      )}
      {error && (
        <span className="recording-control__error" role="alert">
          {error}
        </span>
      )}
    </div>
  );
}
