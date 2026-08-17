/**
 * Extracts a user-safe message from a rejected Tauri `invoke()` call.
 * Commands that fail reject with the `AppError` shape the backend
 * serializes (`{ code, message }` — see docs/architecture.md, "Error
 * handling"); that message is always author-written and safe to show
 * directly, never raw SQL/paths/driver detail.
 */
export function getErrorMessage(error: unknown): string {
  if (
    error &&
    typeof error === "object" &&
    "message" in error &&
    typeof (error as { message: unknown }).message === "string"
  ) {
    return (error as { message: string }).message;
  }
  return "Something went wrong. Please try again.";
}
