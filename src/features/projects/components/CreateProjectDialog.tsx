import { useState, type FormEvent } from "react";
import { Dialog } from "../../../components/ui/Dialog";
import { getErrorMessage } from "../../../utils/errorMessage";
import { createProject, type Project } from "../services/projects";

interface CreateProjectDialogProps {
  onClose: () => void;
  onCreated: (project: Project) => void;
}

export function CreateProjectDialog({ onClose, onCreated }: CreateProjectDialogProps) {
  const [name, setName] = useState("");
  const [description, setDescription] = useState("");
  const [submitting, setSubmitting] = useState(false);
  const [error, setError] = useState<string | null>(null);

  async function handleSubmit(event: FormEvent) {
    event.preventDefault();
    if (submitting) return;

    setSubmitting(true);
    setError(null);
    try {
      const project = await createProject({
        name,
        description: description.trim() === "" ? undefined : description,
      });
      onCreated(project);
    } catch (err) {
      setError(getErrorMessage(err));
      setSubmitting(false);
    }
  }

  return (
    <Dialog title="New project" onClose={onClose} closable={!submitting}>
      <form className="dialog__body" onSubmit={handleSubmit}>
        <div className="field">
          <label className="field__label" htmlFor="project-name">
            Name
          </label>
          <input
            id="project-name"
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
          <label className="field__label" htmlFor="project-description">
            Description
          </label>
          <textarea
            id="project-description"
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
            {submitting ? "Creating…" : "Create project"}
          </button>
        </div>
      </form>
    </Dialog>
  );
}
