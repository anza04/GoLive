import { useState } from "react";
import { Dialog } from "../../../components/ui/Dialog";
import { getErrorMessage } from "../../../utils/errorMessage";
import { deleteProject, type Project } from "../services/projects";

interface DeleteProjectDialogProps {
  project: Project;
  onClose: () => void;
  onDeleted: (id: string) => void;
}

export function DeleteProjectDialog({ project, onClose, onDeleted }: DeleteProjectDialogProps) {
  const [deleting, setDeleting] = useState(false);
  const [error, setError] = useState<string | null>(null);

  async function handleDelete() {
    if (deleting) return;

    setDeleting(true);
    setError(null);
    try {
      await deleteProject(project.id);
      onDeleted(project.id);
    } catch (err) {
      setError(getErrorMessage(err));
      setDeleting(false);
    }
  }

  return (
    <Dialog title="Delete project?" onClose={onClose} closable={!deleting}>
      <div className="dialog__body">
        <p>
          Are you sure you want to delete <strong>{project.name}</strong>? This action
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
