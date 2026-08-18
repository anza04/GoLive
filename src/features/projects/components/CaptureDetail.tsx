import { useEffect, useState } from "react";
import { formatDate } from "../../../utils/formatDate";
import { getErrorMessage } from "../../../utils/errorMessage";
import { CaptureTypeBadge } from "./CaptureTypeBadge";
import { EditCaptureDialog } from "./EditCaptureDialog";
import { getCaptureMediaUrl, type Capture } from "../services/captures";

interface CaptureDetailProps {
  capture: Capture;
  onUpdated: (capture: Capture) => void;
  onDeleteRequested: () => void;
  /** The capture no longer exists (e.g. deleted elsewhere) — the caller
   * drops it from the list and clears selection. */
  onGone: () => void;
}

type MediaState =
  | { state: "none" }
  | { state: "loading" }
  | { state: "ready"; url: string }
  | { state: "error"; message: string };

export function CaptureDetail({ capture, onUpdated, onDeleteRequested, onGone }: CaptureDetailProps) {
  const [editOpen, setEditOpen] = useState(false);
  const [media, setMedia] = useState<MediaState>({ state: "none" });

  // Fetches the PNG fresh whenever the selected capture (or its type)
  // changes — never cached in component state beyond this effect's
  // lifetime, since the object URL must be revoked once it's no longer
  // shown (see `services/captures.ts`, `getCaptureMediaUrl`).
  useEffect(() => {
    if (capture.type !== "screenshot") {
      setMedia({ state: "none" });
      return;
    }

    let cancelled = false;
    let objectUrl: string | null = null;
    setMedia({ state: "loading" });

    void (async () => {
      try {
        const url = await getCaptureMediaUrl(capture.id);
        if (cancelled) {
          URL.revokeObjectURL(url);
          return;
        }
        objectUrl = url;
        setMedia({ state: "ready", url });
      } catch (error) {
        if (!cancelled) {
          setMedia({ state: "error", message: getErrorMessage(error) });
        }
      }
    })();

    return () => {
      cancelled = true;
      if (objectUrl) {
        URL.revokeObjectURL(objectUrl);
      }
    };
  }, [capture.id, capture.type]);

  return (
    <div className="capture-detail">
      <div className="capture-detail__header">
        <div>
          <h4 className="capture-detail__title">{capture.title}</h4>
          {capture.description && (
            <p className="capture-detail__description">{capture.description}</p>
          )}
        </div>
        <div className="workspace__actions">
          <button type="button" className="button" onClick={() => setEditOpen(true)}>
            Edit
          </button>
          <button type="button" className="button button--danger" onClick={onDeleteRequested}>
            Delete
          </button>
        </div>
      </div>

      {capture.type === "screenshot" && (
        <div className="capture-detail__media">
          {media.state === "loading" && <p className="projects-status">Loading screenshot…</p>}
          {media.state === "error" && (
            <p className="dialog__error" role="alert">
              {media.message}
            </p>
          )}
          {media.state === "ready" && (
            <img className="capture-detail__image" src={media.url} alt={capture.title} />
          )}
        </div>
      )}

      <div className="capture-detail__meta">
        <CaptureTypeBadge type={capture.type} />
        <span>Created {formatDate(capture.createdAt)}</span>
        <span>Last updated {formatDate(capture.updatedAt)}</span>
      </div>

      {editOpen && (
        <EditCaptureDialog
          capture={capture}
          onClose={() => setEditOpen(false)}
          onUpdated={(updated) => {
            setEditOpen(false);
            onUpdated(updated);
          }}
          onNotFound={onGone}
        />
      )}
    </div>
  );
}
