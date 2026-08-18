import { useEffect, useState } from "react";
import { EmptyState } from "../../../components/ui/EmptyState";
import { getErrorMessage } from "../../../utils/errorMessage";
import { useActiveProcess } from "../../../stores/activeProcess";
import { ProcessList } from "./ProcessList";
import { ProcessDetail } from "./ProcessDetail";
import { CreateProcessDialog } from "./CreateProcessDialog";
import { DeleteProcessDialog } from "./DeleteProcessDialog";
import { listProcesses, type Process } from "../services/processes";

interface ProcessesViewProps {
  projectId: string;
  /** Only needed to label the active-process store/tray (TASK-010) — the
   * project's own name isn't otherwise used by this view. */
  projectName: string;
}

// Selection stays local to this view, mirroring ProjectsView's original
// `selectedId` approach (before Projects grew a full workspace) — see
// docs/architecture.md ("State management"). Processes live inside an
// already-scoped Project Workspace tab, so a simple list+detail pane is
// deliberately kept instead of a second nested workspace/back-navigation
// layer, per this task's "keep the architecture simple" instruction.
type ListState = { state: "loading" } | { state: "ready" } | { state: "error"; message: string };

export function ProcessesView({ projectId, projectName }: ProcessesViewProps) {
  const [processes, setProcesses] = useState<Process[]>([]);
  const [listState, setListState] = useState<ListState>({ state: "loading" });
  const [selectedId, setSelectedId] = useState<string | null>(null);
  const [createOpen, setCreateOpen] = useState(false);
  const [deleteTarget, setDeleteTarget] = useState<Process | null>(null);
  const { activeProcess, setActiveProcess, clearActiveProcess } = useActiveProcess();

  useEffect(() => {
    void refresh();
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [projectId]);

  async function refresh() {
    setListState({ state: "loading" });
    try {
      const result = await listProcesses(projectId);
      setProcesses(result);
      setListState({ state: "ready" });
    } catch (error) {
      setListState({ state: "error", message: getErrorMessage(error) });
    }
  }

  // The one place all four "this process is now the one on screen"
  // moments (select, create, and — see below — a rename of the already-
  // active process) funnel through, so the active-process store/tray
  // (TASK-010) never drifts from what's actually selected here.
  function markActive(process: Process) {
    setActiveProcess({
      processId: process.id,
      processName: process.name,
      projectId,
      projectName,
    });
  }

  function handleSelect(id: string) {
    setSelectedId(id);
    const process = processes.find((candidate) => candidate.id === id);
    if (process) markActive(process);
  }

  function handleCreated(process: Process) {
    setProcesses((prev) => [process, ...prev]);
    setSelectedId(process.id);
    markActive(process);
    setCreateOpen(false);
  }

  function handleUpdated(updated: Process) {
    // Matches the backend's `updated_at DESC` order without a re-fetch.
    setProcesses((prev) => [updated, ...prev.filter((process) => process.id !== updated.id)]);
    // Keeps the tray's label from going stale if the active process was
    // just renamed.
    if (activeProcess?.processId === updated.id) markActive(updated);
  }

  function handleGone(id: string) {
    setProcesses((prev) => prev.filter((process) => process.id !== id));
    setSelectedId((current) => (current === id ? null : current));
    setDeleteTarget(null);
    if (activeProcess?.processId === id) clearActiveProcess();
  }

  if (listState.state === "loading") {
    return <p className="projects-status">Loading processes…</p>;
  }

  if (listState.state === "error") {
    return (
      <EmptyState
        title="Couldn't load processes"
        description={listState.message}
        action={
          <button type="button" className="button" onClick={() => void refresh()}>
            Retry
          </button>
        }
      />
    );
  }

  if (processes.length === 0) {
    return (
      <>
        <EmptyState
          title="No processes yet"
          description="Create a process to start documenting how this project works."
          action={
            <button
              type="button"
              className="button button--primary"
              onClick={() => setCreateOpen(true)}
            >
              + New process
            </button>
          }
        />
        {createOpen && (
          <CreateProcessDialog
            projectId={projectId}
            onClose={() => setCreateOpen(false)}
            onCreated={handleCreated}
          />
        )}
      </>
    );
  }

  const selectedProcess = processes.find((process) => process.id === selectedId) ?? null;

  return (
    <div className="processes-layout">
      <div className="processes-list-pane">
        <div className="projects-toolbar">
          <button
            type="button"
            className="button button--primary"
            onClick={() => setCreateOpen(true)}
          >
            + New process
          </button>
        </div>
        <ProcessList processes={processes} selectedId={selectedId} onSelect={handleSelect} />
      </div>

      <div className="processes-detail-pane">
        {selectedProcess ? (
          <ProcessDetail
            process={selectedProcess}
            onUpdated={handleUpdated}
            onDeleteRequested={() => setDeleteTarget(selectedProcess)}
            onGone={() => handleGone(selectedProcess.id)}
          />
        ) : (
          <EmptyState
            title="No process selected"
            description="Select a process from the list to see its details."
          />
        )}
      </div>

      {createOpen && (
        <CreateProcessDialog
          projectId={projectId}
          onClose={() => setCreateOpen(false)}
          onCreated={handleCreated}
        />
      )}
      {deleteTarget && (
        <DeleteProcessDialog
          process={deleteTarget}
          onClose={() => setDeleteTarget(null)}
          onDeleted={handleGone}
        />
      )}
    </div>
  );
}
