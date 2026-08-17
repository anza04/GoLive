import { useState, type FormEvent } from "react";
import { Dialog } from "../../../components/ui/Dialog";
import { getErrorMessage } from "../../../utils/errorMessage";
import { createProcess, type Process } from "../services/processes";

interface CreateProcessDialogProps {
  projectId: string;
  onClose: () => void;
  onCreated: (process: Process) => void;
}

/** Status defaults to Draft — the backend establishes it, the dialog
 * never asks the user to specify it on create. */
export function CreateProcessDialog({ projectId, onClose, onCreated }: CreateProcessDialogProps) {
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
      const process = await createProcess({
        projectId,
        name,
        description: description.trim() === "" ? undefined : description,
      });
      onCreated(process);
    } catch (err) {
      setError(getErrorMessage(err));
      setSubmitting(false);
    }
  }

  return (
    <Dialog title="New process" onClose={onClose} closable={!submitting}>
      <form className="dialog__body" onSubmit={handleSubmit}>
        <div className="field">
          <label className="field__label" htmlFor="process-name">
            Name
          </label>
          <input
            id="process-name"
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
          <label className="field__label" htmlFor="process-description">
            Description
          </label>
          <textarea
            id="process-description"
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
            {submitting ? "Creating…" : "Create process"}
          </button>
        </div>
      </form>
    </Dialog>
  );
}
