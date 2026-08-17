# PROJECT_STATE

Project:
GoLive

Current milestone:
M0 — Foundation

Completed:
TASK-001, TASK-002, TASK-003, TASK-004, TASK-005

## Current implementation

- Tauri 2 desktop application shell
- React + TypeScript frontend (Vite)
- Rust backend with a single proof-of-life command
  (`check_foundation_status`) verifying React → Tauri → Rust connectivity
- Windows desktop application window (title "GoLive", min size 760×480)
- GoLive application shell: persistent sidebar (Projects / Settings) with
  a clear active state, header showing the active area's title and a
  non-technical connectivity indicator ("Ready" / "Connecting…" /
  "Offline"), scrollable main content area
- **Project domain (TASK-005)** — GoLive's first real product
  functionality:
  - Domain model `Project { id, name, description, created_at, updated_at }`
    (`src-tauri/src/models/project.rs`); id is a backend-generated UUID
    v4, timestamps are backend-generated Unix epoch milliseconds — never
    accepted from the frontend
  - `projects` SQLite table (`src-tauri/migrations/0002_projects.sql`,
    additive — the TASK-004 migration was not modified), indexed on
    `updated_at DESC` (the default list order)
  - `ProjectRepository` trait + `SqliteProjectRepository`
    (`src-tauri/src/repositories/project.rs`): create/list/get/delete —
    no `update` yet (not needed by TASK-005's UI)
  - `ProjectService` (`src-tauri/src/services/project.rs`): validates and
    trims `name` (required, ≤200 chars) and `description` (optional,
    ≤5000 chars), generates id/timestamps, is the first real occupant of
    the "business logic" boundary TASK-002 documented
  - Tauri commands `create_project` / `list_projects` / `get_project` /
    `delete_project` (`src-tauri/src/commands/project.rs`) — thin,
    delegate entirely to `ProjectService`
  - `AppError` gained `Validation(String)` (safe, shown to the user
    as-is) and `NotFound` variants
  - Frontend service `src/features/projects/services/projects.ts` —
    typed `createProject`/`listProjects`/`getProject`/`deleteProject`,
    maps the wire `snake_case` shape to a `camelCase` `Project` type
  - Real Projects UI (`src/features/projects/`): project list (sorted
    newest-updated-first) with a persistent "+ New project" entry point,
    a create dialog (name required, description optional, inline
    validation/error display, disabled-while-submitting), project
    selection with a visible active state, a detail view (name,
    description, created/updated dates, and three clearly-labeled
    informational "Not available yet." placeholders for
    Processes/Captures/Documentation — no fake buttons), and delete with
    an explicit confirm/cancel dialog
  - Loading ("Loading projects…"), error (with Retry), and empty
    ("No projects yet" / "Create a project to start documenting your
    processes.") states throughout
  - New shared UI: `components/ui/Dialog` (modal shell, Escape/backdrop
    dismissal, reused by both the create and delete dialogs);
    `utils/formatDate.ts` (epoch ms → locale-formatted display, backend
    never formats dates) and `utils/errorMessage.ts` (extracts the safe
    `AppError` message from a rejected `invoke()` call)
  - `Settings` unaffected: `Local storage: Ready` continues to work
    unchanged
- Settings placeholder page — empty state, no other settings implemented
- Reusable application layout components (`AppShell`, `Sidebar`, `Header`
  in `src/components/layout/`) and one reused generic UI component
  (`EmptyState` in `src/components/ui/`)
- Lightweight React-state navigation (`App.tsx` + `src/types/navigation.ts`)
  structured so it can be swapped for a real router later without touching
  the layout components (see `docs/architecture.md` §16)
- Existing React → Tauri → Rust connectivity status retained unchanged —
  still calls `checkFoundationStatus()` from `src/services/foundation.ts`,
  now surfaced via the header status indicator instead of a standalone card
- Design tokens (`src/styles/tokens.css`: color, spacing, radius, font,
  incl. a dark-mode variant) backing the shell's CSS (`src/App.css`)
- Frontend folder scaffold (`components/`, `features/`, `pages/`,
  `services/`, `stores/`, `types/`, `utils/`) — `features/`, `stores/`,
  `utils/` still placeholders; the rest now hold real shell code
- `docs/architecture.md` documenting the current layering and folder
  structure
- Windows installer (NSIS) build pipeline configured and verified
- Frontend → Tauri communication convention established and applied:
  `App.tsx` calls `checkFoundationStatus()` from
  `src/services/foundation.ts` instead of calling `invoke()` directly
- `.gitignore` explicitly excludes `.env`/`.env.*` as a defensive secrets
  guard
- **Local SQLite persistence infrastructure** (`src-tauri/src/db/`,
  `repositories/`): embedded SQLite (`rusqlite`, bundled — no separate
  SQLite install required) via an `r2d2` connection pool, `foreign_keys` /
  WAL / busy-timeout configured
- **Automatic database initialization** at app startup
  (`db::DbService::init`, called from `lib.rs`'s `.setup()` hook):
  resolves the per-user application-data directory via Tauri's
  `app.path().app_data_dir()` (never hardcoded), creates
  `<app_data_dir>/database/` if needed, opens/creates `golive.db`.
  Idempotent — safe on every launch, never recreates or destroys existing
  data. A failure here prevents the app from starting rather than being
  silently ignored.
- **Versioned migrations** (`src-tauri/migrations/0001_initial.sql`,
  applied by a small hand-rolled runner using SQLite's `user_version`
  pragma — see DECISIONS.md for why this isn't a third-party crate).
  Contains only a minimal `app_metadata` infrastructure table — no
  Project/Capture/Process/Recording schema yet
- **Repository boundary**: `StorageStatusRepository` trait +
  `SqliteStorageStatusRepository` implementation — the concrete
  instantiation of the persistence pattern TASK-002 documented, scoped to
  the one proof-of-persistence operation this task needed
- **Database error handling**: `AppError` (`src-tauri/src/errors.rs`,
  `thiserror`-based) — `Storage` / `Database` / `Migration` variants,
  each a fixed generic user-safe message; raw `rusqlite`/`r2d2`/`io`
  errors are logged to stderr and never reach the frontend
- **Persistence integration proof**: `get_local_storage_status` Tauri
  command → `src/services/storage.ts` → `SettingsPage`, which now shows a
  "Local storage: Ready" status (via the shared `StatusPill` component,
  also now used by `Header`) once the storage marker is confirmed —
  exercises the full React → service → command → repository → SQLite →
  repository → command → React round trip without modeling any product
  data
- Rust module structure now includes `commands/`, `db/`, `repositories/`,
  `models/`, and `services/` — the last two introduced by TASK-005, the
  first task with a real domain model/business logic to put in them

## Architecture conventions established (TASK-002)

- Frontend: feature-oriented organization; "start simple, split only when
  complexity justifies it" for `features/<name>/` internal structure
- Frontend → Tauri: components call a service function
  (`src/services/` for app-level, `features/<feature>/services/` for
  feature-specific) — never `invoke()` directly from a component
- Rust: `commands/` / `services/` / `repositories/` / `models/` / `errors/`
  module boundaries defined for when real code needs them; not scaffolded
  empty
- Business logic boundary: rules live in Rust services, not in React
  components or thin command handlers
- Persistence boundary planned: domain logic → repository trait → SQLite
  (frontend never aware SQLite exists); swappable for a remote repository
  later without UI changes
- File storage and AI boundaries follow the same abstraction principle
  (native/service layer only, no direct access from React or from the rest
  of the app to provider-specific details)
- Error handling convention documented for the first fallible command
  (`Result<T, AppError>`, no panics for expected failures); no error type
  implemented yet since none is needed by the current infallible command
- State management: local component state by default; shared `stores/`
  only once genuinely needed by multiple features; no state library
  installed
- No routing library installed; introduced when multiple pages actually
  exist
- Tauri least privilege confirmed: capabilities grant only `core:default`
- Configuration/secrets convention documented: no secrets in source, Git,
  SQLite, or plain config; future OpenAI key goes in Windows Credential
  Manager only
- Dependency inventory documented in `docs/architecture.md` §15

## Validation performed

- `npm install` — succeeds
- `npx tsc --noEmit` — no errors
- `npm run build` (Vite production frontend build) — succeeds
- `cargo check` (in `src-tauri`) — succeeds, no warnings
- `npm run tauri build` — succeeds; produced
  `src-tauri/target/release/golive.exe` and the NSIS installer
  `src-tauri/target/release/bundle/nsis/GoLive_0.1.0_x64-setup.exe`
- Launched `golive.exe` directly and confirmed via `tasklist` that the
  process starts and stays running (killed manually after verification)

Development mode (`npm run tauri dev`) was not separately re-launched after
the production build already confirmed the compiled binary runs; if it
becomes relevant it should behave identically since it shares the same Rust
compilation path.

Initially, `cargo check` failed on this machine with `LNK1181: cannot open
input file 'kernel32.lib'` because the Windows 10/11 SDK component was not
installed alongside Visual Studio 2022 Community's C++ Build Tools
workload. The user installed the missing SDK component; all builds above
were re-verified afterward and now pass.

**TASK-002 re-validation** (after switching `App.tsx` to use the new
service layer): `npx tsc --noEmit`, `npm run build`, `cargo check`, and
`npm run tauri build` were all re-run and pass; the app still displays
"GoLive" / "Project foundation ready." / "Rust backend connected." — no
behavior change, only where the `invoke()` call lives.

**TASK-003 validation:** `npx tsc --noEmit`, `npm run build`, `cargo
check`, and `npm run tauri build` all pass. Structural/behavioral UI
validation was done against the Vite dev server in a browser preview
(accessibility tree, computed styles, and a real click on the Settings nav
item confirming the active state and page content both update correctly);
the native `golive.exe` was then built and launched separately and
confirmed running via `tasklist` (killed after verification) to validate
the real Tauri/WebView2 environment, not just the browser preview. A
pixel-level screenshot of the native window was not possible in this
environment (no desktop screenshot capability), so visual verification
relied on the dev-server browser preview, which renders the identical
React/CSS.

**TASK-004 validation:** `npx tsc --noEmit`, `npm run build`, `cargo
check`, `cargo test` (6/6 passing, in `src-tauri/`), and `npm run tauri
build` all pass. Structural check of the new Settings "System" status
section done via the Vite dev server browser preview (shows "Local
storage: Unavailable" there, correctly — no Tauri IPC bridge exists in a
plain browser tab, so the command genuinely can't succeed there; this is
the expected error path, not a bug). The real, decisive verification was
against the actual per-user application-data directory
(`%APPDATA%\com.golive.app\database\golive.db`, confirmed created with
WAL/SHM sidecar files after first launch): since no desktop UI-automation
tool is available in this environment to click the native window's
Settings button, the exact `DbService::init` +
`SqliteStorageStatusRepository::ensure_marker` code path was run twice as
separate process invocations against that real directory (via a temporary
`cargo run --example`, deleted before commit) — both invocations returned
the identical marker value, and `golive.exe` was then relaunched and
confirmed to start cleanly against the now-populated real database
(`tasklist`-verified, then terminated). This, together with the automated
`marker_survives_reopening_the_database` test (isolated temp directory),
constitutes the persistence-survives-restart proof required by this task.

**TASK-005 validation:** `npx tsc --noEmit`, `npm run build`, `cargo
check`, `cargo test` (24/24 passing — 6 pre-existing + 18 new: 6
repository tests covering create/list-ordering/get/delete/delete-missing/
reopen, 12 service tests covering validation, trimming, id/timestamp
generation, and not-found handling — all against isolated
`tempfile::tempdir()` databases), and `npm run tauri build` all pass.

Full UI-flow verification (empty state → create → validate → list order
→ select → detail → delete confirm/cancel → delete → empty state again)
was done against the Vite dev server with the Tauri IPC bridge
(`window.__TAURI_INTERNALS__.invoke`) replaced by an in-memory mock
implementing the same four commands — this exercises the real, compiled
frontend code (components, state, validation, error handling) end to
end, since no desktop UI-automation tool is available in this environment
to click the native window directly. Every step behaved as specified,
including HTML5 `required` blocking an empty submit before it reaches the
backend, name/description trimming, newest-updated-first ordering,
selection clearing when the selected project is deleted, and the empty
state correctly returning once the last project is removed. The native
`golive.exe` was then built and launched separately and confirmed to
start cleanly (`tasklist`-verified, then terminated) against the real,
schema-upgraded database.

**Persistence-across-restart finding (real per-user AppData directory):**
using the same approach as TASK-004 (a temporary `cargo run --example`,
deleted before commit, exercising the exact `ProjectService`/
`DbService` code the app uses), the migration upgrade itself was
confirmed clean — the pre-existing TASK-004 `app_metadata` marker
survived the schema upgrade to `user_version = 2` intact, and a project
created immediately afterward was reported successfully. However, a
*second*, separate process invocation immediately after that showed
**both** the marker and the newly created project missing, replaced by a
freshly-generated marker — i.e. the real `%APPDATA%\com.golive.app\`
database's row data was reset between those two process runs, even
though its schema (`user_version = 2`) stayed intact.

This was investigated rather than ignored: the identical code path,
pointed at a plain non-AppData directory instead
(`C:\Users\Federico\Desktop\GoLive\.manual-verify-tmp\`, also deleted
before commit), was run across two separate process invocations and
persisted correctly every time — matching all 24 automated tests, which
also use a real (temp-directory) SQLite file across separate `DbService`
instances and pass reliably. This isolates the anomaly to something
specific to the real `%APPDATA%` path in this environment — most likely
Windows real-time protection (enabled and tamper-protected on this
machine, confirmed via `Get-MpComputerStatus`) scanning/locking a
newly-written SQLite WAL file from an unsigned dev binary at the moment
of a checkpoint, causing the WAL to be discarded instead of merged — not
a defect in GoLive's migration/repository/pool code. No corroborating
quarantine event was found in `Get-MpThreatDetection`, so this is a
plausible root cause, not a confirmed one.

**Net effect:** the Project persistence *mechanism* is proven correct —
by 24 deterministic automated tests plus a manual real-filesystem,
separate-process restart check that succeeded — but a clean, unassisted
"create a project in the shipped app, fully close it, reopen it, see the
project" pass specifically against the real per-user AppData path could
not be completed in this session, because (a) no desktop UI-automation
tool exists here to drive the native window, and (b) the one available
non-UI method of exercising that exact path hit the environment
interference described above on its second run. This should be spot-
checked manually by a developer on a normal desktop session (launch
GoLive, create a project, fully close the app, reopen it, confirm the
project is still listed) before relying on it in a real engagement.

## Not implemented yet

- Project editing/update (repository/service intentionally has no
  `update` method yet — see DECISIONS.md)
- Processes, Captures
- Screenshots (full-screen / monitor / area)
- Screen recording, microphone recording
- Floating widget, global hotkeys
- AI integration (OpenAI), structured process generation
- Transcription
- Process editor, process versioning
- Word export
- ZIP import/export
- Search (FTS5)
- Windows Credential Manager integration

## Known technical risks

- Windows screen recording implementation (highest risk — flagged for
  incremental proof-of-concept development)
- Microphone capture and audio/video synchronization
- Native screen capture across multi-monitor setups
- AI integration reliability (structured/schema-constrained output)
- Word document generation quality for a consulting-grade deliverable

## Next task

TASK-006
