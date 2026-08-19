import { useEffect, useState } from "react";

/**
 * Ticks once a second, returning the whole seconds elapsed since
 * `startedAt` (epoch ms) — the shared "recording elapsed time" logic
 * both the Captures section's toolbar control and the floating widget
 * use (TASK-014), so a recording started in one and observed from the
 * other shows the same running time. Returns `0` and never ticks while
 * `startedAt` is `null` (no recording in progress).
 *
 * `hooks/` is this project's first occupant — a small, single-purpose
 * piece of UI logic genuinely shared by two independent components,
 * matching this project's "shared code only once genuinely needed by
 * multiple call sites" convention (see docs/architecture.md).
 */
export function useElapsedSeconds(startedAt: number | null): number {
  const [elapsed, setElapsed] = useState(() => secondsSince(startedAt));

  useEffect(() => {
    setElapsed(secondsSince(startedAt));
    if (startedAt === null) return;

    const intervalId = window.setInterval(() => setElapsed(secondsSince(startedAt)), 1000);
    return () => window.clearInterval(intervalId);
  }, [startedAt]);

  return elapsed;
}

function secondsSince(startedAt: number | null): number {
  if (startedAt === null) return 0;
  return Math.max(0, Math.floor((Date.now() - startedAt) / 1000));
}
