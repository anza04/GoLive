import { createContext, useCallback, useContext, useMemo, useState, type ReactNode } from "react";
import { syncActiveProcess, type ActiveProcessInfo } from "../services/activeProcess";

/**
 * The currently "active" Process — GoLive's first genuinely
 * cross-feature client state (see docs/architecture.md, "State
 * management"). A plain React Context, not a state-management library:
 * this is exactly the "same-shape swap into a stores/ slice"
 * ProjectsView's original `activeProject` comment anticipated, not a
 * reason to add a dependency.
 *
 * Purely a client-side notion in *this* (main) window — nothing here is
 * persisted, and there is no backend model for it (see
 * docs/architecture.md). Two things outside this window's React tree
 * read it: the system tray, and (TASK-011) the floating capture widget,
 * a separate window with no access to this Context — both are kept in
 * sync via `services/activeProcess.ts`'s `syncActiveProcess`, called as
 * a side effect of `setActiveProcess`/`clearActiveProcess` so callers
 * here never have to remember to do it themselves.
 */
export type ActiveProcess = ActiveProcessInfo;

interface ActiveProcessContextValue {
  activeProcess: ActiveProcess | null;
  setActiveProcess: (process: ActiveProcess) => void;
  clearActiveProcess: () => void;
}

const ActiveProcessContext = createContext<ActiveProcessContextValue | null>(null);

/** Mounted once, in `App.tsx`, above everything that needs to read or
 * write the active process. */
export function ActiveProcessProvider({ children }: { children: ReactNode }) {
  const [activeProcess, setActiveProcessState] = useState<ActiveProcess | null>(null);

  const setActiveProcess = useCallback((process: ActiveProcess) => {
    setActiveProcessState(process);
    void syncActiveProcess(process);
  }, []);

  const clearActiveProcess = useCallback(() => {
    setActiveProcessState(null);
    void syncActiveProcess(null);
  }, []);

  const value = useMemo(
    () => ({ activeProcess, setActiveProcess, clearActiveProcess }),
    [activeProcess, setActiveProcess, clearActiveProcess],
  );

  return <ActiveProcessContext.Provider value={value}>{children}</ActiveProcessContext.Provider>;
}

/** Throws if used outside `ActiveProcessProvider` — a programming error
 * (a missing Provider in the tree), not a runtime state callers need to
 * handle. */
export function useActiveProcess(): ActiveProcessContextValue {
  const context = useContext(ActiveProcessContext);
  if (!context) {
    throw new Error("useActiveProcess must be used within an ActiveProcessProvider");
  }
  return context;
}
