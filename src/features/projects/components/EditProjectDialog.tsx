import { useState, type FormEvent } from "react";
import { Dialog } from "../../../components/ui/Dialog";
import { getErrorMessage, isNotFoundError } from "../../../utils/errorMessage";
import { updateProject, type Project } from "../services/projects";

interface EditProjectDialogProps {
  project: Project;
  onClose: () => void;
  onUpdated: (project: Project) => void;
  /** The project was deleted elsewhere between opening the workspace and
   * saving — exit gracefully instead of showing a generic error. */
  onNotFound: () => void;
}

export function EditProjectDialog({ project, onClose, onUpdated, onNotFound }: EditProjectDialogProps) {
  const [name, setName] = useState(project.name);
  const [description, setDescription] = useState(project.description);
  const [submitting, setSubmitting] = useState(false);
  const [error, setError] = useState<string | null>(null);

  async function handleSubmit(event: FormEvent) {
    event.preventDefault();
    if (submitting) return;

    setSubmitting(true);
    setError(null);
    try {
      const updated = await updateProject({
        id: project.id,
        name,
        description: description.trim() === "" ? undefined : description,
      });
      onUpdated(updated);
    } catch (err) {
      if (isNotFoundError(err)) {
        onNotFound();
        return;
      }
      // Cancel discards local changes by simply not calling the backend;
      // on error we deliberately keep the dialog open with what the user
      // typed still in `name`/`description` — nothing is reverted.
      setError(getErrorMessage(err));
      setSubmitting(false);
    }
  }

  return (
    <Dialog title="Edit project" onClose={onClose} closable={!submitting}>
      <form className="dialog__body" onSubmit={handleSubmit}>
        <div className="field">
          <label className="field__label" htmlFor="edit-project-name">
            Name
          </label>
          <input
            id="edit-project-name"
            className="field__input"
            value={name}
            onChange={(event) => setName(event.target.value)}
            maxLength={200}
            autoFocus
            required
            disabled={submitting}
          />
        </div>

        <div className="field">
          <label className="field__label" htmlFor="edit-project-description">
            Description
          </label>
          <textarea
            id="edit-project-description"
            className="field__textarea"
            value={description}
            onChange={(event) => setDescription(event.target.value)}
            maxLength={5000}
            rows={3}
            disabled={submitting}
          />
        </div>

        {error && (
          <p className="dialog__error" role="alert">
            {error}
          </p>
        )}

        <div className="dialog__footer">
          <button type="button" className="button" onClick={onClose} disabled={submitting}>
            Cancel
          </button>
          <button type="submit" className="button button--primary" disabled={submitting}>
            {submitting ? "Saving…" : "Save"}
          </button>
        </div>
      </form>
    </Dialog>
  );
}
