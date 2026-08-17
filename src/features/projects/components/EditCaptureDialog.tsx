import { useState, type FormEvent } from "react";
import { Dialog } from "../../../components/ui/Dialog";
import { getErrorMessage, isNotFoundError } from "../../../utils/errorMessage";
import { updateCapture, type Capture, type CaptureType } from "../services/captures";

interface EditCaptureDialogProps {
  capture: Capture;
  onClose: () => void;
  onUpdated: (capture: Capture) => void;
  /** The capture was deleted elsewhere between opening it and saving —
   * exit gracefully instead of showing a generic error. */
  onNotFound: () => void;
}

const TYPE_OPTIONS: { value: CaptureType; label: string }[] = [
  { value: "screenshot", label: "Screenshot" },
  { value: "recording", label: "Recording" },
  { value: "note", label: "Note" },
];

export function EditCaptureDialog({ capture, onClose, onUpdated, onNotFound }: EditCaptureDialogProps) {
  const [title, setTitle] = useState(capture.title);
  const [description, setDescription] = useState(capture.description);
  const [captureType, setCaptureType] = useState<CaptureType>(capture.type);
  const [submitting, setSubmitting] = useState(false);
  const [error, setError] = useState<string | null>(null);

  async function handleSubmit(event: FormEvent) {
    event.preventDefault();
    if (submitting) return;

    setSubmitting(true);
    setError(null);
    try {
      const updated = await updateCapture({
        id: capture.id,
        captureType,
        title,
        description: description.trim() === "" ? undefined : description,
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
    <Dialog title="Edit capture" onClose={onClose} closable={!submitting}>
      <form className="dialog__body" onSubmit={handleSubmit}>
        <div className="field">
          <label className="field__label" htmlFor="edit-capture-title">
            Title
          </label>
          <input
            id="edit-capture-title"
            className="field__input"
            value={title}
            onChange={(event) => setTitle(event.target.value)}
            maxLength={200}
            autoFocus
            required
            disabled={submitting}
          />
        </div>

        <div className="field">
          <label className="field__label" htmlFor="edit-capture-description">
            Description
          </label>
          <textarea
            id="edit-capture-description"
            className="field__textarea"
            value={description}
            onChange={(event) => setDescription(event.target.value)}
            maxLength={5000}
            rows={3}
            disabled={submitting}
          />
        </div>

        <div className="field">
          <label className="field__label" htmlFor="edit-capture-type">
            Type
          </label>
          <select
            id="edit-capture-type"
            className="field__input"
            value={captureType}
            onChange={(event) => setCaptureType(event.target.value as CaptureType)}
            disabled={submitting}
          >
            {TYPE_OPTIONS.map((option) => (
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
