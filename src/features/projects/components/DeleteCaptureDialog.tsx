import { useState } from "react";
import { Dialog } from "../../../components/ui/Dialog";
import { getErrorMessage, isNotFoundError } from "../../../utils/errorMessage";
import { deleteCapture, type Capture } from "../services/captures";

interface DeleteCaptureDialogProps {
  capture: Capture;
  onClose: () => void;
  onDeleted: (id: string) => void;
}

export function DeleteCaptureDialog({ capture, onClose, onDeleted }: DeleteCaptureDialogProps) {
  const [deleting, setDeleting] = useState(false);
  const [error, setError] = useState<string | null>(null);

  async function handleDelete() {
    if (deleting) return;

    setDeleting(true);
    setError(null);
    try {
      await deleteCapture(capture.id);
      onDeleted(capture.id);
    } catch (err) {
      if (isNotFoundError(err)) {
        // Already gone — that's the exact state a successful delete
        // leaves you in, so treat it the same as success.
        onDeleted(capture.id);
        return;
      }
      setError(getErrorMessage(err));
      setDeleting(false);
    }
  }

  return (
    <Dialog title="Delete capture?" onClose={onClose} closable={!deleting}>
      <div className="dialog__body">
        <p>
          Are you sure you want to delete <strong>{capture.title}</strong>? This action
          cannot be undone.
        </p>

        {error && (
          <p className="dialog__error" role="alert">
            {error}
          </p>
        )}

        <div className="dialog__footer">
          <button type="button" className="button" onClick={onClose} disabled={deleting}>
            Cancel
          </button>
          <button
            type="button"
            className="button button--danger"
            onClick={handleDelete}
            disabled={deleting}
          >
            {deleting ? "Deleting…" : "Delete"}
          </button>
        </div>
      </div>
    </Dialog>
  );
}
