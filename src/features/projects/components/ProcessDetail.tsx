import { useState } from "react";
import { formatDate } from "../../../utils/formatDate";
import { ProcessStatusBadge } from "./ProcessStatusBadge";
import { EditProcessDialog } from "./EditProcessDialog";
import { CapturesSection } from "./CapturesSection";
import type { Process } from "../services/processes";

interface ProcessDetailProps {
  process: Process;
  onUpdated: (process: Process) => void;
  onDeleteRequested: () => void;
  /** The process no longer exists (e.g. deleted elsewhere) — the caller
   * drops it from the list and clears selection. */
  onGone: () => void;
}

// Informational-only preview of what a process will eventually own
// (TASK-009+: recordings, transcripts, AI analysis). No
// clickable/functional-looking controls. "Captures" used to be here too —
// TASK-008 replaced it with the real `CapturesSection` below.
const RESERVED_SECTIONS = [
  { title: "AI analysis", description: "AI-assisted process analysis will be available here." },
] as const;

export function ProcessDetail({ process, onUpdated, onDeleteRequested, onGone }: ProcessDetailProps) {
  const [editOpen, setEditOpen] = useState(false);

  return (
    <div className="process-detail">
      <div className="process-detail__header">
        <div>
          <h3 className="process-detail__name">{process.name}</h3>
          {process.description && (
            <p className="process-detail__description">{process.description}</p>
          )}
        </div>
        <div className="workspace__actions">
          <button type="button" className="button" onClick={() => setEditOpen(true)}>
            Edit
          </button>
          <button type="button" className="button button--danger" onClick={onDeleteRequested}>
            Delete
          </button>
        </div>
      </div>

      <div className="process-detail__meta">
        <ProcessStatusBadge status={process.status} />
        <span>Created {formatDate(process.createdAt)}</span>
        <span>Last updated {formatDate(process.updatedAt)}</span>
      </div>

      <CapturesSection processId={process.id} />

      <div className="reserved-sections">
        {RESERVED_SECTIONS.map((section) => (
          <div className="reserved-section" key={section.title}>
            <h4 className="reserved-section__title">{section.title}</h4>
            <p className="reserved-section__hint">{section.description}</p>
          </div>
        ))}
      </div>

      {editOpen && (
        <EditProcessDialog
          process={process}
          onClose={() => setEditOpen(false)}
          onUpdated={(updated) => {
            setEditOpen(false);
            onUpdated(updated);
          }}
          onNotFound={onGone}
        />
      )}
    </div>
  );
}
