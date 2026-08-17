import type { ReactNode } from "react";

interface AppShellProps {
  sidebar: ReactNode;
  header: ReactNode;
  children: ReactNode;
}

/**
 * Top-level page frame: a persistent sidebar, a header for the active
 * area, and a scrollable content region. Purely structural — it owns no
 * navigation or connectivity state itself, so it has no dependency on how
 * either is implemented.
 */
export function AppShell({ sidebar, header, children }: AppShellProps) {
  return (
    <div className="app-shell">
      <aside className="app-shell__sidebar">{sidebar}</aside>
      <div className="app-shell__body">
        <header className="app-shell__header">{header}</header>
        <main className="app-shell__content">{children}</main>
      </div>
    </div>
  );
}
