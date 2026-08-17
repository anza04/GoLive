# utils

Small, pure, framework-agnostic helper functions shared across the frontend.

- `formatDate.ts` — formats a Unix-epoch-milliseconds backend timestamp
  for human display in the user's local locale/timezone. Backend
  timestamps are always the machine representation; formatting happens
  only here, never in SQLite or Rust.
- `errorMessage.ts` — extracts the safe `message` from a rejected Tauri
  `invoke()` call's `AppError` shape, with a generic fallback.
