import { useState, type FormEvent } from "react";
import { Dialog } from "../../../components/ui/Dialog";
import { getErrorMessage } from "../../../utils/errorMessage";
import {
  createCapture,
  createScreenshotCapture,
  startRecordingCapture,
  stopRecordingCapture,
  type Capture,
  type CaptureType,
} from "../services/captures";

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
  // Only meaningful while captureType === "recording": whether
  // `start_recording_capture` has already succeeded, i.e. a real
  // recording is currently in progress on the backend and the primary
  // button's next click should *stop* it rather than start a new one
  // (TASK-013's two-phase flow — see `services/captures.ts`).
  const [recordingStarted, setRecordingStarted] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const isScreenshot = captureType === "screenshot";
  const isRecording = captureType === "recording";
  // Once a real recording has started, the request that started it
  // (title/description/type) can no longer be changed or abandoned via
  // Cancel — doing either would leave an in-progress native recording
  // that nothing ever stops. The user must click through to Stop; the
  // orphan-cleanup sweep (`MediaStorage::reconcile`) is the safety net
  // if the whole app is closed before that happens (see DECISIONS.md).
  const fieldsLocked = submitting || recordingStarted;

  async function handleSubmit(event: FormEvent) {
    event.preventDefault();
    if (submitting) return;

    setSubmitting(true);
    setError(null);
    try {
      if (isRecording && !recordingStarted) {
        const trimmedDescription = description.trim() === "" ? undefined : description;
        await startRecordingCapture({ processId, title, description: trimmedDescription });
        setRecordingStarted(true);
        setSubmitting(false);
        return;
      }

      if (isRecording && recordingStarted) {
        const capture = await stopRecordingCapture();
        onCreated(capture);
        return;
      }

      const trimmedDescription = description.trim() === "" ? undefined : description;
      // Screenshot goes through the dedicated real-media operation
      // (captures the Windows display, stores the PNG, then creates the
      // metadata) — Note stays the existing metadata-only path. There's
      // no `captureType` field to pass to the screenshot operation at
      // all: it always produces `type: "screenshot"`.
      const capture = isScreenshot
        ? await createScreenshotCapture({ processId, title, description: trimmedDescription })
        : await createCapture({ processId, captureType, title, description: trimmedDescription });
      onCreated(capture);
    } catch (err) {
      setError(getErrorMessage(err));
      setSubmitting(false);
    }
  }

  function primaryButtonLabel(): string {
    if (isScreenshot) return submitting ? "Capturing…" : "Capture screenshot";
    if (isRecording) {
      if (recordingStarted) return submitting ? "Stopping…" : "Stop recording";
      return submitting ? "Starting…" : "Start recording";
    }
    return submitting ? "Creating…" : "Create capture";
  }

  return (
    <Dialog title="New capture" onClose={onClose} closable={!submitting && !recordingStarted}>
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
            disabled={fieldsLocked}
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
            disabled={fieldsLocked}
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
            disabled={fieldsLocked}
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
          {isRecording && (
            <p className="field__hint">
              {recordingStarted
                ? "Recording your primary display — click Stop recording when you're done."
                : "This will record your primary display until you click Stop recording."}
            </p>
          )}
        </div>

        {error && (
          <p className="dialog__error" role="alert">
            {error}
          </p>
        )}

        <div className="dialog__footer">
          <button type="button" className="button" onClick={onClose} disabled={submitting || recordingStarted}>
            Cancel
          </button>
          <button type="submit" className="button button--primary" disabled={submitting}>
            {primaryButtonLabel()}
          </button>
        </div>
      </form>
    </Dialog>
  );
}
