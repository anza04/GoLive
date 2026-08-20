import { useEffect, useState } from "react";
import { EmptyState } from "../../../components/ui/EmptyState";
import { getErrorMessage } from "../../../utils/errorMessage";
import { CaptureList } from "./CaptureList";
import { CaptureDetail } from "./CaptureDetail";
import { CreateCaptureDialog } from "./CreateCaptureDialog";
import { DeleteCaptureDialog } from "./DeleteCaptureDialog";
import { NewCaptureMenu } from "./NewCaptureMenu";
import {
  defaultRecordingTitle,
  getRecordingStatus,
  listCaptures,
  onCaptureCreated,
  onRecordingStatusChanged,
  startRecordingCapture,
  stopRecordingCapture,
  type Capture,
  type CaptureType,
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
  const [createType, setCreateType] = useState<CaptureType>("screenshot");
  const [deleteTarget, setDeleteTarget] = useState<Capture | null>(null);

  // Recording is system-wide, not scoped to this Process (see
  // docs/architecture.md, "one recording at a time") — this status is
  // fetched once and kept live via `onRecordingStatusChanged` for the
  // component's whole lifetime, independent of which Process is
  // currently selected (TASK-014).
  const [recordingStatus, setRecordingStatus] = useState<RecordingStatus | null>(null);
  const [recordingBusy, setRecordingBusy] = useState(false);
  const [recordingError, setRecordingError] = useState<string | null>(null);
  // Opt-in microphone toggle (TASK-015) — plain local state, not
  // persisted, read once when "Start recording" is clicked (see
  // DECISIONS.md).
  const [includeAudio, setIncludeAudio] = useState(false);

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

    const unlistenRecording = onRecordingStatusChanged((status) => setRecordingStatus(status));

    // Bugfix (see DECISIONS.md): a Capture created from the floating
    // widget — a hotkey screenshot, a quick marker, a recording stopped
    // there — used to never appear here until the section was
    // remounted, since nothing told this list to refetch. `handleCreated`
    // dedupes by id, so this is safe even for a capture this same
    // window already added locally (dialog submit, this section's own
    // Stop recording).
    const unlistenCreated = onCaptureCreated((capture) => {
      if (capture.processId === processId) handleCreated(capture);
    });

    return () => {
      cancelled = true;
      void unlistenRecording.then((fn) => fn());
      void unlistenCreated.then((fn) => fn());
    };
    // `processId` is read fresh inside the listener closure below via
    // this effect's own dependency — re-subscribing on change keeps the
    // filter correct without a stale closure.
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [processId]);

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
    setCaptures((prev) => (prev.some((existing) => existing.id === capture.id) ? prev : [capture, ...prev]));
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

  function handleCreateScreenshot() {
    setCreateType("screenshot");
    setCreateOpen(true);
  }

  function handleCreateNote() {
    setCreateType("note");
    setCreateOpen(true);
  }

  async function handleStartRecording() {
    if (recordingBusy) return;
    setRecordingBusy(true);
    setRecordingError(null);
    try {
      const status = await startRecordingCapture({ processId, title: defaultRecordingTitle(), includeAudio });
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

  const newCaptureMenu = (
    <NewCaptureMenu
      onCreateScreenshot={handleCreateScreenshot}
      onCreateNote={handleCreateNote}
      recordingStatus={recordingStatus}
      processId={processId}
      recordingBusy={recordingBusy}
      recordingError={recordingError}
      includeAudio={includeAudio}
      onIncludeAudioChange={setIncludeAudio}
      onStartRecording={() => void handleStartRecording()}
      onStopRecording={() => void handleStopRecording()}
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
        <div className="projects-toolbar">{newCaptureMenu}</div>
        <EmptyState
          title="No captures yet"
          description="Create a capture to start collecting evidence for this process."
        />
        {createOpen && (
          <CreateCaptureDialog
            processId={processId}
            initialType={createType}
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
          <div className="projects-toolbar">{newCaptureMenu}</div>
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
          initialType={createType}
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
