import { useState } from "react";
import { formatDate } from "../../../utils/formatDate";
import { CaptureTypeBadge } from "./CaptureTypeBadge";
import { EditCaptureDialog } from "./EditCaptureDialog";
import type { Capture } from "../services/captures";

interface CaptureDetailProps {
  capture: Capture;
  onUpdated: (capture: Capture) => void;
  onDeleteRequested: () => void;
  /** The capture no longer exists (e.g. deleted elsewhere) — the caller
   * drops it from the list and clears selection. */
  onGone: () => void;
}

export function CaptureDetail({ capture, onUpdated, onDeleteRequested, onGone }: CaptureDetailProps) {
  const [editOpen, setEditOpen] = useState(false);

  return (
    <div className="capture-detail">
      <div className="capture-detail__header">
        <div>
          <h4 className="capture-detail__title">{capture.title}</h4>
          {capture.description && (
            <p className="capture-detail__description">{capture.description}</p>
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

      <div className="capture-detail__meta">
        <CaptureTypeBadge type={capture.type} />
        <span>Created {formatDate(capture.createdAt)}</span>
        <span>Last updated {formatDate(capture.updatedAt)}</span>
      </div>

      {editOpen && (
        <EditCaptureDialog
          capture={capture}
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
