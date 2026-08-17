/**
 * Formats a Unix-epoch-milliseconds timestamp for display in the user's
 * local timezone/locale. Backend timestamps are always the machine
 * representation (epoch ms, UTC) — formatting for humans happens only
 * here, never in SQLite or Rust.
 */
export function formatDate(epochMs: number): string {
  return new Date(epochMs).toLocaleString(undefined, {
    dateStyle: "medium",
    timeStyle: "short",
  });
}
