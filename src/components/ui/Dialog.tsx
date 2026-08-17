import { useEffect, useId, type ReactNode } from "react";

interface DialogProps {
  title: string;
  onClose: () => void;
  children: ReactNode;
  /** Set false to disable Escape/backdrop-click dismissal — e.g. while a
   * submission is in flight, so it can't be dismissed mid-request. */
  closable?: boolean;
}

/** Minimal modal dialog shell: overlay, panel, Escape-to-close,
 * click-outside-to-close, proper `role="dialog"` semantics. Generic on
 * purpose — used for both the create and delete project dialogs. */
export function Dialog({ title, onClose, children, closable = true }: DialogProps) {
  const titleId = useId();

  useEffect(() => {
    if (!closable) return;
    function onKeyDown(event: KeyboardEvent) {
      if (event.key === "Escape") onClose();
    }
    document.addEventListener("keydown", onKeyDown);
    return () => document.removeEventListener("keydown", onKeyDown);
  }, [closable, onClose]);

  return (
    <div
      className="dialog-overlay"
      onMouseDown={(event) => {
        if (closable && event.target === event.currentTarget) onClose();
      }}
    >
      <div className="dialog" role="dialog" aria-modal="true" aria-labelledby={titleId}>
        <h2 id={titleId} className="dialog__title">
          {title}
        </h2>
        {children}
      </div>
    </div>
  );
}
