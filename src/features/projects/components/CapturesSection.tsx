import { useEffect, useState } from "react";
import { EmptyState } from "../../../components/ui/EmptyState";
import { getErrorMessage } from "../../../utils/errorMessage";
import { CaptureList } from "./CaptureList";
import { CaptureDetail } from "./CaptureDetail";
import { CreateCaptureDialog } from "./CreateCaptureDialog";
import { DeleteCaptureDialog } from "./DeleteCaptureDialog";
import { listCaptures, type Capture } from "../services/captures";

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

  useEffect(() => {
    void refresh();
    setSelectedId(null);
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

  const newCaptureButton = (
    <button type="button" className="button button--primary" onClick={() => setCreateOpen(true)}>
      + New capture
    </button>
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
        <EmptyState
          title="No captures yet"
          description="Create a capture to start collecting evidence for this process."
          action={newCaptureButton}
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
