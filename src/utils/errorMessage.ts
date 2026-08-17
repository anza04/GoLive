interface RawAppError {
  code: string;
  message: string;
}

function isAppError(error: unknown): error is RawAppError {
  return (
    !!error &&
    typeof error === "object" &&
    "code" in error &&
    typeof (error as RawAppError).code === "string" &&
    "message" in error &&
    typeof (error as RawAppError).message === "string"
  );
}

/**
 * Extracts a user-safe message from a rejected Tauri `invoke()` call.
 * Commands that fail reject with the `AppError` shape the backend
 * serializes (`{ code, message }` — see docs/architecture.md, "Error
 * handling"); that message is always author-written and safe to show
 * directly, never raw SQL/paths/driver detail.
 */
export function getErrorMessage(error: unknown): string {
  return isAppError(error) ? error.message : "Something went wrong. Please try again.";
}

/**
 * True if a rejected `invoke()` call failed because the record no longer
 * exists (`AppError::NotFound`, code `"not_found"`) — e.g. it was deleted
 * elsewhere between opening the workspace and saving an edit. Callers use
 * this to exit gracefully instead of showing a generic error.
 */
export function isNotFoundError(error: unknown): boolean {
  return isAppError(error) && error.code === "not_found";
}
