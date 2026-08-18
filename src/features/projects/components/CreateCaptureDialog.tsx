import { useState, type FormEvent } from "react";
import { Dialog } from "../../../components/ui/Dialog";
import { getErrorMessage } from "../../../utils/errorMessage";
import { createCapture, createScreenshotCapture, type Capture, type CaptureType } from "../services/captures";

interface CreateCaptureDialogProps {
  processId: string;
  onClose: () => void;
  onCreated: (capture: Capture) => void;
}

const TYPE_OPTIONS: { value: CaptureType; label: string }[] = [
  { value: "screenshot", label: "Screenshot" },
  { value: "recording", label: "Recording" },
  { value: "note", label: "Note" },
];

/** Status defaults to Screenshot — the most common capture, and the type
 * that actually does something when the primary action is pressed. */
export function CreateCaptureDialog({ processId, onClose, onCreated }: CreateCaptureDialogProps) {
  const [title, setTitle] = useState("");
  const [description, setDescription] = useState("");
  const [captureType, setCaptureType] = useState<CaptureType>("screenshot");
  const [submitting, setSubmitting] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const isScreenshot = captureType === "screenshot";

  async function handleSubmit(event: FormEvent) {
    event.preventDefault();
    if (submitting) return;

    setSubmitting(true);
    setError(null);
    try {
      const trimmedDescription = description.trim() === "" ? undefined : description;
      // Screenshot goes through the dedicated real-media operation
      // (captures the Windows display, stores the PNG, then creates the
      // metadata) — every other type stays the existing metadata-only
      // path. There's no `captureType` field to pass to the screenshot
      // operation at all: it always produces `type: "screenshot"`.
      const capture = isScreenshot
        ? await createScreenshotCapture({ processId, title, description: trimmedDescription })
        : await createCapture({ processId, captureType, title, description: trimmedDescription });
      onCreated(capture);
    } catch (err) {
      setError(getErrorMessage(err));
      setSubmitting(false);
    }
  }

  return (
    <Dialog title="New capture" onClose={onClose} closable={!submitting}>
      <form className="dialog__body" onSubmit={handleSubmit}>
        <div className="field">
          <label className="field__label" htmlFor="capture-title">
            Title
          </label>
          <input
            id="capture-title"
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
          <label className="field__label" htmlFor="capture-description">
            Description
          </label>
          <textarea
            id="capture-description"
            className="field__textarea"
            value={description}
            onChange={(event) => setDescription(event.target.value)}
            maxLength={5000}
            rows={3}
            disabled={submitting}
          />
        </div>

        <div className="field">
          <label className="field__label" htmlFor="capture-type">
            Type
          </label>
          <select
            id="capture-type"
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
          {isScreenshot && (
            <p className="field__hint">This will capture your current screen.</p>
          )}
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
            {submitting ? (isScreenshot ? "Capturing…" : "Creating…") : isScreenshot ? "Capture screenshot" : "Create capture"}
          </button>
        </div>
      </form>
    </Dialog>
  );
}
