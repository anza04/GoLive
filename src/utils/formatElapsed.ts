/**
 * Formats a whole-second duration as `M:SS`, or `H:MM:SS` once it
 * passes an hour — the recording in-progress indicator's display format
 * (TASK-014). Unlike `formatDate`, this isn't locale-aware — a stopwatch
 * reads the same everywhere.
 */
export function formatElapsed(totalSeconds: number): string {
  const clamped = Math.max(0, Math.floor(totalSeconds));
  const hours = Math.floor(clamped / 3600);
  const minutes = Math.floor((clamped % 3600) / 60);
  const seconds = clamped % 60;
  const paddedSeconds = String(seconds).padStart(2, "0");

  if (hours > 0) {
    const paddedMinutes = String(minutes).padStart(2, "0");
    return `${hours}:${paddedMinutes}:${paddedSeconds}`;
  }
  return `${minutes}:${paddedSeconds}`;
}
