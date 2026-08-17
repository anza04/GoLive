import { useState, type FormEvent } from "react";
import { Dialog } from "../../../components/ui/Dialog";
import { getErrorMessage, isNotFoundError } from "../../../utils/errorMessage";
import { updateProcess, type Process, type ProcessStatus } from "../services/processes";

interface EditProcessDialogProps {
  process: Process;
  onClose: () => void;
  onUpdated: (process: Process) => void;
  /** The process was deleted elsewhere between opening it and saving —
   * exit gracefully instead of showing a generic error. */
  onNotFound: () => void;
}

const STATUS_OPTIONS: { value: ProcessStatus; label: string }[] = [
  { value: "draft", label: "Draft" },
  { value: "in_progress", label: "In progress" },
  { value: "completed", label: "Completed" },
];

export function EditProcessDialog({ process, onClose, onUpdated, onNotFound }: EditProcessDialogProps) {
  const [name, setName] = useState(process.name);
  const [description, setDescription] = useState(process.description);
  const [status, setStatus] = useState<ProcessStatus>(process.status);
  const [submitting, setSubmitting] = useState(false);
  const [error, setError] = useState<string | null>(null);

  async function handleSubmit(event: FormEvent) {
    event.preventDefault();
    if (submitting) return;

    setSubmitting(true);
    setError(null);
    try {
      const updated = await updateProcess({
        id: process.id,
        name,
        description: description.trim() === "" ? undefined : description,
        status,
      });
      onUpdated(updated);
    } catch (err) {
      if (isNotFoundError(err)) {
        onNotFound();
        return;
      }
      setError(getErrorMessage(err));
      setSubmitting(false);
    }
  }

  return (
    <Dialog title="Edit process" onClose={onClose} closable={!submitting}>
      <form className="dialog__body" onSubmit={handleSubmit}>
        <div className="field">
          <label className="field__label" htmlFor="edit-process-name">
            Name
          </label>
          <input
            id="edit-process-name"
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
          <label className="field__label" htmlFor="edit-process-description">
            Description
          </label>
          <textarea
            id="edit-process-description"
            className="field__textarea"
            value={description}
            onChange={(event) => setDescription(event.target.value)}
            maxLength={5000}
            rows={3}
            disabled={submitting}
          />
        </div>

        <div className="field">
          <label className="field__label" htmlFor="edit-process-status">
            Status
          </label>
          <select
            id="edit-process-status"
            className="field__input"
            value={status}
            onChange={(event) => setStatus(event.target.value as ProcessStatus)}
            disabled={submitting}
          >
            {STATUS_OPTIONS.map((option) => (
              <option key={option.value} value={option.value}>
                {option.label}
              </option>
            ))}
          </select>
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
