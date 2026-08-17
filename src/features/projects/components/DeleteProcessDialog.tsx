import { useState } from "react";
import { Dialog } from "../../../components/ui/Dialog";
import { getErrorMessage, isNotFoundError } from "../../../utils/errorMessage";
import { deleteProcess, type Process } from "../services/processes";

interface DeleteProcessDialogProps {
  process: Process;
  onClose: () => void;
  onDeleted: (id: string) => void;
}

export function DeleteProcessDialog({ process, onClose, onDeleted }: DeleteProcessDialogProps) {
  const [deleting, setDeleting] = useState(false);
  const [error, setError] = useState<string | null>(null);

  async function handleDelete() {
    if (deleting) return;

    setDeleting(true);
    setError(null);
    try {
      await deleteProcess(process.id);
      onDeleted(process.id);
    } catch (err) {
      if (isNotFoundError(err)) {
        // Already gone — that's the exact state a successful delete
        // leaves you in, so treat it the same as success.
        onDeleted(process.id);
        return;
      }
      setError(getErrorMessage(err));
      setDeleting(false);
    }
  }

  return (
    <Dialog title="Delete process?" onClose={onClose} closable={!deleting}>
      <div className="dialog__body">
        <p>
          Are you sure you want to delete <strong>{process.name}</strong>? This action
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
