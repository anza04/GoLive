# services

Frontend-side wrappers around Tauri `invoke()` calls. The React layer talks
to Rust exclusively through functions in here — never through a direct
`invoke()` call inside a component. See `docs/architecture.md`
("Frontend -> Tauri communication") for the full convention.

This top-level `services/` folder is for **app-level** calls that don't
belong to one feature (e.g. `foundation.ts`, the backend connectivity
check). Once a feature has its own Tauri commands, its service functions
should live in `features/<feature>/services/` instead, next to the rest of
that feature's code, not here.

## Current contents

- `foundation.ts` — wraps the `check_foundation_status` proof-of-life
  command.
- `storage.ts` — wraps `get_local_storage_status`, the SQLite
  persistence proof-of-life command (TASK-004). No SQLite-specific
  terminology crosses this boundary — it returns a plain
  `{ ready, initializedAt }` shape.

Project's Tauri calls are *not* here — they're feature-specific, so they
live in `features/projects/services/projects.ts` instead (see
`features/README.md`), per the convention above.
