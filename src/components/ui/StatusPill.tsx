type StatusTone = "neutral" | "ok" | "error";

interface StatusPillProps {
  tone: StatusTone;
  label: string;
  /** Optional technical detail, shown only as a hover tooltip — never in
   * the visible label (see docs/architecture.md, error handling). */
  detail?: string;
}

/** Small colored-dot + label status indicator. Generic on purpose: used
 * for backend connectivity in `Header` and for local storage status in
 * `SettingsPage`. Never shows technical/implementation detail in the
 * visible label. */
export function StatusPill({ tone, label, detail }: StatusPillProps) {
  return (
    <div className={`status-pill status-pill--${tone}`} role="status" title={detail}>
      <span className="status-pill__dot" aria-hidden="true" />
      <span>{label}</span>
    </div>
  );
}
