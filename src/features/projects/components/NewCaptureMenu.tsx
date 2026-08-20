import { useEffect, useRef, useState } from "react";
import { formatElapsed } from "../../../utils/formatElapsed";
import { useElapsedSeconds } from "../../../hooks/useElapsedSeconds";
import type { RecordingStatus } from "../services/captures";

interface NewCaptureMenuProps {
  onCreateScreenshot: () => void;
  onCreateNote: () => void;
  recordingStatus: RecordingStatus | null;
  processId: string;
  recordingBusy: boolean;
  recordingError: string | null;
  includeAudio: boolean;
  onIncludeAudioChange: (value: boolean) => void;
  onStartRecording: () => void;
  onStopRecording: () => void;
}

/**
 * The Captures section's single creation entry point — a bugfix/redesign
 * (see DECISIONS.md, "toolbar hierarchy"): TASK-014/015 had put "+ New
 * capture", an always-visible "Include microphone audio" checkbox, and a
 * "Start recording" button side by side in one toolbar row, which (a)
 * read as three unrelated, competing actions instead of one clear
 * "create a capture" affordance, and (b) was wide enough to overflow a
 * narrower window (see docs/architecture.md §29). Consolidated into one
 * "+ New capture" button that opens a small popover offering all three
 * creation paths — Screenshot and Note still go through
 * `CreateCaptureDialog`; Recording's audio checkbox and Start action
 * live *inside* the popover instead of occupying their own toolbar
 * slots.
 *
 * Once a recording is actually in progress (for this Process), the
 * trigger is replaced by the elapsed-time/Stop indicator directly — at
 * that point there's nothing left to pick from a menu for, and the
 * in-progress state needs to stay glanceable, not hidden behind a click.
 */
export function NewCaptureMenu({
  onCreateScreenshot,
  onCreateNote,
  recordingStatus,
  processId,
  recordingBusy,
  recordingError,
  includeAudio,
  onIncludeAudioChange,
  onStartRecording,
  onStopRecording,
}: NewCaptureMenuProps) {
  const [open, setOpen] = useState(false);
  const containerRef = useRef<HTMLDivElement>(null);

  const isRecordingHere = recordingStatus?.processId === processId;
  const isRecordingElsewhere = recordingStatus !== null && !isRecordingHere;
  const elapsedSeconds = useElapsedSeconds(isRecordingHere ? recordingStatus.startedAt : null);

  useEffect(() => {
    if (!open) return;

    function handlePointerDown(event: MouseEvent) {
      if (containerRef.current && !containerRef.current.contains(event.target as Node)) {
        setOpen(false);
      }
    }
    function handleKeyDown(event: KeyboardEvent) {
      if (event.key === "Escape") setOpen(false);
    }

    document.addEventListener("mousedown", handlePointerDown);
    document.addEventListener("keydown", handleKeyDown);
    return () => {
      document.removeEventListener("mousedown", handlePointerDown);
      document.removeEventListener("keydown", handleKeyDown);
    };
  }, [open]);

  if (isRecordingHere) {
    return (
      <div className="new-capture-menu">
        <div className="recording-indicator">
          <span className="recording-indicator__dot" role="status">
            ● Recording {formatElapsed(elapsedSeconds)}
          </span>
          <button type="button" className="button button--danger" onClick={onStopRecording} disabled={recordingBusy}>
            {recordingBusy ? "Stopping…" : "Stop recording"}
          </button>
        </div>
        {recordingError && (
          <span className="recording-indicator__error" role="alert">
            {recordingError}
          </span>
        )}
      </div>
    );
  }

  return (
    <div className="new-capture-menu" ref={containerRef}>
      <button
        type="button"
        className="button button--primary"
        onClick={() => setOpen((current) => !current)}
        aria-haspopup="menu"
        aria-expanded={open}
      >
        + New capture
      </button>

      {open && (
        <div className="new-capture-menu__popover" role="menu">
          <button
            type="button"
            className="new-capture-menu__item"
            role="menuitem"
            onClick={() => {
              setOpen(false);
              onCreateScreenshot();
            }}
          >
            Screenshot
          </button>
          <button
            type="button"
            className="new-capture-menu__item"
            role="menuitem"
            onClick={() => {
              setOpen(false);
              onCreateNote();
            }}
          >
            Note
          </button>

          <div className="new-capture-menu__divider" />

          <label className="new-capture-menu__audio-toggle">
            <input
              type="checkbox"
              checked={includeAudio}
              onChange={(event) => onIncludeAudioChange(event.target.checked)}
              disabled={recordingBusy || isRecordingElsewhere}
            />
            Include microphone audio
          </label>
          <button
            type="button"
            className="new-capture-menu__item"
            role="menuitem"
            onClick={() => {
              setOpen(false);
              onStartRecording();
            }}
            disabled={recordingBusy || isRecordingElsewhere}
            title={isRecordingElsewhere ? "A recording is already in progress for another process." : undefined}
          >
            {recordingBusy ? "Starting…" : "Start recording"}
          </button>

          {recordingError && (
            <span className="new-capture-menu__error" role="alert">
              {recordingError}
            </span>
          )}
        </div>
      )}
    </div>
  );
}
