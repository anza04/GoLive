type ConnectivityState = "checking" | "ok" | "error";

interface HeaderProps {
  title: string;
  status: ConnectivityState;
  /** Optional technical detail, shown only as a hover tooltip — never in
   * the visible status text (see docs/architecture.md, error handling). */
  statusDetail?: string;
}

const STATUS_LABEL: Record<ConnectivityState, string> = {
  checking: "Connecting…",
  ok: "Ready",
  error: "Offline",
};

/** Application header: shows the active area's title and a subtle,
 * user-facing (non-technical) backend connectivity indicator. */
export function Header({ title, status, statusDetail }: HeaderProps) {
  return (
    <div className="header">
      <h1 className="header__title">{title}</h1>
      <div
        className={`header__status header__status--${status}`}
        role="status"
        title={statusDetail}
      >
        <span className="header__status-dot" aria-hidden="true" />
        <span>{STATUS_LABEL[status]}</span>
      </div>
    </div>
  );
}
