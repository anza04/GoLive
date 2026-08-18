# PROJECT_STATE

Project:
GoLive

Current milestone:
M0 — Foundation

Completed:
TASK-001, TASK-002, TASK-003, TASK-004, TASK-005, TASK-006, TASK-007,
TASK-008, TASK-009

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
- **Project domain (TASK-005) + Project Workspace and editing
  (TASK-006)** — GoLive's first real product functionality:
  - Domain model `Project { id, name, description, created_at, updated_at }`
    (`src-tauri/src/models/project.rs`); id is a backend-generated UUID
    v4, timestamps are backend-generated Unix epoch milliseconds — never
    accepted from the frontend, including through update
  - `projects` SQLite table (`src-tauri/migrations/0002_projects.sql`,
    additive — the TASK-004 migration was not modified), indexed on
    `updated_at DESC` (the default list order); no further migration was
    needed for update (existing columns already sufficient)
  - `ProjectRepository` trait + `SqliteProjectRepository`
    (`src-tauri/src/repositories/project.rs`): create/list/get/**update**/
    delete — `update` writes `name`/`description`/`updated_at` only,
    keyed by `id`; `id`/`created_at` are structurally outside the SQL
    `SET` clause, not just a convention
  - `ProjectService` (`src-tauri/src/services/project.rs`): `create` and
    **`update`** both validate/trim `name` (required, ≤200 chars) and
    `description` (optional, ≤5000 chars); `update` additionally verifies
    the project exists and regenerates `updated_at` while carrying
    `id`/`created_at` forward from the existing record
  - Tauri commands `create_project` / `list_projects` / `get_project` /
    **`update_project`** / `delete_project`
    (`src-tauri/src/commands/project.rs`) — thin, delegate entirely to
    `ProjectService`. `update_project` takes an explicit
    `UpdateProjectInput { id, name, description }`, not the `Project`
    model itself, so a request has no field for `created_at`/`updated_at`
    to occupy
  - `AppError` gained `Validation(String)` (safe, shown to the user
    as-is) and `NotFound` variants (TASK-005; unchanged this task)
  - Frontend service `src/features/projects/services/projects.ts` —
    typed `createProject`/`listProjects`/`getProject`/**`updateProject`**/
    `deleteProject`, maps the wire `snake_case` shape to a `camelCase`
    `Project` type
  - **Project Workspace** (`src/features/projects/components/ProjectWorkspace.tsx`):
    selecting a project in the list opens its workspace (back button,
    name/description header, Edit/Delete actions, a tab bar with
    "Overview" implemented and "Processes"/"Captures"/"Documentation"
    genuinely `disabled` — not fake-clickable). `ProjectOverview.tsx`
    shows created/updated dates plus three informational "Not available
    yet." cards. `EditProjectDialog.tsx` (pre-filled name/description,
    Cancel discards with no backend call, Save persists and immediately
    updates the workspace + moves the project to the top of the list).
    Delete moved from the old list-adjacent detail pane into the
    workspace (confirm dialog unchanged), clearing the active project and
    returning to Projects on success
  - `activeProject` (`ProjectsView.tsx`, `useState<Project | null>`)
    replaces TASK-005's `selectedId` — holds the full record the
    workspace needs, still feature-local, still not a global store
  - Projects list is now list-only (no more inline two-pane detail);
    still has its own loading/error/empty states, "+ New project" entry
    point, and creation dialog, unchanged from TASK-005
  - Shared UI: `components/ui/Dialog` (modal shell, reused by all three
    project dialogs); `utils/formatDate.ts`; `utils/errorMessage.ts`
    (TASK-006 added `isNotFoundError()` for graceful NotFound handling)
  - `Settings` unaffected: `Local storage: Ready` continues to work
    unchanged
- **Process domain (TASK-007)** — GoLive's second real domain entity,
  related 1:N to Project (`Project 1 ─── N Process`), nested inside the
  Project Workspace:
  - Domain model `Process { id, project_id, name, description, status,
    created_at, updated_at }` (`src-tauri/src/models/process.rs`);
    `status` is a real `ProcessStatus` Rust enum (`Draft` / `InProgress` /
    `Completed`) implementing `rusqlite`'s `ToSql`/`FromSql` directly and
    serializing as one of three stable lowercase strings — the same
    representation in SQLite, the Rust wire format, and the frontend
    `ProcessStatus` TypeScript union
  - `processes` SQLite table (`src-tauri/migrations/0003_processes.sql`,
    additive — `0001`/`0002` untouched):
    `project_id TEXT NOT NULL REFERENCES projects(id) ON DELETE CASCADE`,
    indexed on `project_id` and `(project_id, updated_at DESC)`.
    **Deleting a Project cascades to delete its Processes at the database
    level** — no application-side loop, proven by an automated test
  - `ProcessRepository` trait + `SqliteProcessRepository`
    (`src-tauri/src/repositories/process.rs`): create/list_by_project
    (scoped to one project, `updated_at DESC`)/get/update/delete — same
    shape as `ProjectRepository`; `update`'s SQL never touches `id`,
    `project_id`, or `created_at`
  - `ProcessService` (`src-tauri/src/services/process.rs`) holds *both*
    `ProcessRepository` and `ProjectRepository`: `create` confirms the
    parent project exists (clean `NotFound` instead of relying solely on
    the foreign key) before writing, and always sets `status: Draft` —
    the frontend cannot specify status on create. `update` validates/
    trims name/description (same 200/5000 limits as Project) and parses
    the status string via `ProcessStatus::parse`, rejecting anything
    else with `AppError::Validation`
  - Tauri commands `create_process` / `list_processes` / `get_process` /
    `update_process` / `delete_process`
    (`src-tauri/src/commands/process.rs`) — thin, delegate to
    `ProcessService`. `list_processes` takes an explicit
    `ListProcessesInput { project_id }` (not a bare parameter) and
    `update_process` an explicit `UpdateProcessInput { id, name,
    description, status }` — no field for `project_id`/`created_at`/
    `updated_at` to occupy
  - No new `AppError` variants needed — reuses `Validation`/`NotFound`
  - Frontend service `src/features/projects/services/processes.ts` —
    typed `createProcess`/`listProcesses`/`getProcess`/`updateProcess`/
    `deleteProcess`
  - **Processes tab activated** in `ProjectWorkspace` (was disabled since
    TASK-006): tabs are now real state (`activeTab`, was a fixed
    constant), with an `available` flag per tab — Overview and Processes
    enabled, Captures/Documentation still genuinely `disabled`.
    `ProjectWorkspace` is now mounted with `key={project.id}` so
    switching projects resets the active tab to Overview
  - `ProcessesView` (the Processes tab's content): list + detail pane
    (deliberately simpler than the Project Workspace's own
    list→full-workspace pattern — Processes doesn't get a second nested
    workspace), its own loading/error/empty states, "+ New process"
    entry point, `ProcessList` (name, `ProcessStatusBadge`, updated
    date), `ProcessDetail` (name, description, status, dates, Edit/
    Delete, and two reserved informational cards — "Captures" / "AI
    analysis"), `CreateProcessDialog` (no status field), `EditProcessDialog`
    (adds a status `<select>`), `DeleteProcessDialog` — all reuse the
    shared `components/ui/Dialog`
  - `ProjectOverview`'s reserved-sections list dropped "Processes" — no
    longer a placeholder
- **Capture domain (TASK-008)** — GoLive's third real domain entity,
  related 1:N to Process (`Project 1 ─── N Process 1 ─── N Capture`),
  nested inside a selected Process rather than a new Workspace tab.
  Metadata only — no screenshot/recording/media file is captured or
  stored by this task:
  - Domain model `Capture { id, process_id, type, title, description,
    created_at, updated_at }` (`src-tauri/src/models/capture.rs`);
    `type` is a real `CaptureType` Rust enum (`Screenshot` / `Recording` /
    `Note`) implementing `rusqlite`'s `ToSql`/`FromSql` directly and
    serializing as one of three stable lowercase strings — the same
    representation in SQLite, the Rust wire format, and the frontend
    `CaptureType` TypeScript union (same pattern as `ProcessStatus`)
  - `captures` SQLite table (`src-tauri/migrations/0004_captures.sql`,
    additive — `0001`–`0003` untouched):
    `process_id TEXT NOT NULL REFERENCES processes(id) ON DELETE CASCADE`,
    indexed on `process_id` and `(process_id, updated_at DESC)`.
    **Deleting a Process cascades to delete its Captures at the database
    level**, and because `processes.project_id` already cascades from
    `projects`, deleting a Project transitively cascades through two
    chained foreign keys — Project → Process → Capture — with no
    application-side cascade loop, proven by automated tests
  - `CaptureRepository` trait + `SqliteCaptureRepository`
    (`src-tauri/src/repositories/capture.rs`): create/list_by_process
    (scoped to one process, `updated_at DESC`)/get/update/delete — same
    shape as `ProcessRepository`; `update`'s SQL never touches `id`,
    `process_id`, or `created_at`
  - `CaptureService` (`src-tauri/src/services/capture.rs`) holds *both*
    `CaptureRepository` and `ProcessRepository`: `create` confirms the
    parent process exists (clean `NotFound` instead of relying solely on
    the foreign key) before writing. `title` (required, ≤200 chars) and
    `description` (optional, ≤5000 chars) use the same trim/validate
    rules as elsewhere; `type` is parsed via `CaptureType::parse`,
    rejecting anything else with `AppError::Validation`
  - Tauri commands `create_capture` / `list_captures` / `get_capture` /
    `update_capture` / `delete_capture`
    (`src-tauri/src/commands/capture.rs`) — thin, delegate to
    `CaptureService`. `list_captures` takes an explicit
    `ListCapturesInput { process_id }` and `update_capture` an explicit
    `UpdateCaptureInput { id, capture_type, title, description }` — no
    `process_id`/`created_at`/`updated_at` field for a request to occupy
  - No new `AppError` variants needed — reuses `Validation`/`NotFound`
  - Frontend service `src/features/projects/services/captures.ts` —
    typed `createCapture`/`listCaptures`/`getCapture`/`updateCapture`/
    `deleteCapture`
  - `ProcessDetail`'s old "Captures" *reserved* informational card is
    gone — replaced by the real `CapturesSection` component (list +
    detail pane, its own loading/error/empty states, "+ New capture"
    entry point), rendered directly inside `ProcessDetail`. Deliberately
    not a second nested Workspace — one level deeper than `ProcessesView`,
    same list+detail shape, scoped to the selected process
  - `CaptureList` (title, `CaptureTypeBadge`, updated date), `CaptureDetail`
    (title, description, type, dates, Edit/Delete), `CaptureTypeBadge`
    (`Screenshot`/`Recording`/`Note`), `CreateCaptureDialog` (Type
    defaults to Screenshot), `EditCaptureDialog` (pre-filled),
    `DeleteCaptureDialog` — all reuse the shared `components/ui/Dialog`
  - The Project Workspace's own "Captures" tab stays genuinely `disabled`
    — Captures belong to a Process, not directly to a Project; no
    project-wide capture list was added
- **Real screenshot capture (TASK-009)** — the first task with real
  media. A Screenshot Capture now has an actual PNG behind it; Note and
  Recording Captures remain metadata only:
  - `native::screenshot::ScreenshotEngine` trait +
    `WindowsScreenshotEngine` (`src-tauri/src/native/screenshot.rs`):
    captures the primary Windows display via the `xcap` crate and
    encodes it to PNG in memory — the Native Windows functionality
    boundary's first real occupant. No monitor picker, area/window
    selection, or recording — deliberately scoped to "capture the
    primary/current display" only
  - `media::MediaStorage` (`src-tauri/src/media/mod.rs`): the file
    storage boundary's first real occupant. Owns
    `<app_data_dir>/captures/` (sibling of `<app_data_dir>/database/`,
    same Tauri-resolved app-data directory, no second mechanism
    invented). `save_capture`/`read_capture`/`delete_capture`/`exists`/
    `reconcile`, all keyed by `Capture.id` alone — the id is validated as
    a UUID before any path is built, which is the entire path-traversal
    defense (a UUID string structurally cannot contain `../`, an
    absolute path, or any unsafe character)
  - **No database migration** — the TASK-008 `captures` schema was
    already sufficient (metadata only, as intentionally designed); the
    PNG is filesystem data, never stored in SQLite
  - `CaptureService::create_screenshot` (new): validates title/
    description, confirms the parent Process exists, captures the
    screen, saves the PNG, then creates the metadata row — orchestrated
    (not a DB transaction, since the media is filesystem data) so that a
    capture-engine failure leaves nothing behind, and a metadata-insert
    failure after a successful PNG write cleans that PNG back up
  - `CaptureService::delete` (extended): deletes metadata first, then
    best-effort deletes the media file — graceful no-op if there wasn't
    one (Note/Recording), never surfaces a raw filesystem error
  - `CaptureService::reconcile_media` (new) + `CaptureRepository::
    list_all_ids` (new): a startup sweep (called once from `lib.rs`'s
    `.setup()`) that removes any `.png` file under `captures/` whose
    Capture id no longer exists in the database — the documented answer
    to Project/Process cascade deletes, which only ever removed
    *metadata* rows (SQLite has no idea files exist). **Documented
    limitation:** a cascade-orphaned PNG is removed at next app startup,
    not the instant the cascade happens; direct Capture deletion (the
    common case) remains synchronous
  - New Tauri commands `create_screenshot_capture` (explicit
    `CreateScreenshotInput { process_id, title, description }` — no
    `capture_type` field at all; a screenshot operation can only ever
    produce `type: "screenshot"`) and `get_capture_media` (returns
    `tauri::ipc::Response` — raw bytes, not a JSON array — keyed only by
    Capture id; the frontend never sends or receives a filesystem path)
  - One new `AppError` variant, `Capture(String)` (native capture-engine
    failures — no display available, PNG encode failure); filesystem
    failures storing media reuse the existing `Storage` variant
  - New dependency: `xcap` (`image` feature) — see docs/architecture.md
    §15 for the full why/alternatives-considered entry
  - Frontend service additions
    (`features/projects/services/captures.ts`): `createScreenshotCapture`
    and `getCaptureMediaUrl` (fetches PNG bytes, returns a `blob:` object
    URL, documents that the caller must revoke it)
  - `CreateCaptureDialog`: when Type is Screenshot, shows a "This will
    capture your current screen." hint and the primary button reads
    "Capture screenshot"/"Capturing…", calling the new screenshot
    operation instead of the generic metadata-only one; Note/Recording
    behavior is unchanged from TASK-008
  - `CaptureDetail`: for `type === "screenshot"`, fetches and shows the
    PNG (loading/error states, bounded/aspect-preserving `<img>`) between
    the description and the type/date row; Note/Recording render nothing
    new
  - Editing a Screenshot Capture's type away from `screenshot` (existing
    generic Edit dialog) does **not** delete or move its PNG — the file
    is keyed by `Capture.id`, not `capture_type`, so it simply becomes
    unreachable through the UI until/unless the type is changed back
    (documented decision, not an oversight — see DECISIONS.md)
  - Project Workspace's "Captures" tab remains genuinely `disabled` —
    untouched by this task
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
- **Versioned migrations** (`src-tauri/migrations/`: `0001_initial.sql`
  — infrastructure-only `app_metadata` table; `0002_projects.sql` —
  `projects`; `0003_processes.sql` — `processes` with a
  `project_id ... ON DELETE CASCADE` foreign key to `projects`; applied
  by a small hand-rolled runner using SQLite's `user_version` pragma —
  see DECISIONS.md for why this isn't a third-party crate). No
  Capture/Recording schema yet
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

**TASK-006 validation:** `npx tsc --noEmit`, `npm run build`, `cargo
check` (no warnings), `cargo test` (44/44 passing — 6 infra + 38 Project
domain tests, up from 24: 4 new repository tests
`update_changes_name_and_description`,
`update_does_not_change_id_or_created_at`,
`update_missing_project_returns_false`,
`updating_a_project_moves_it_to_the_top_of_the_list`, and 6 new service
tests for update trimming/validation/`updated_at` regeneration/
`created_at` preservation/not-found), and `npm run tauri build` all pass.

Full UI-flow verification (empty → create → enters workspace directly →
Overview shows dates + reserved Processes/Captures/Documentation cards →
Edit opens with current values pre-filled → Cancel discards (verified:
typed change to the name field, clicked Cancel, confirmed both the UI and
the mock backend record were unchanged) → Edit again → Save persists
(name changed, `created_at` preserved, `updated_at` changed) and the
workspace shows the new name immediately → "← Projects" returns to the
list showing the updated name → created a second project and edited the
first again to confirm it moves back to the top of the list on update →
Delete from the workspace requires confirmation → deleting returns to
Projects with the item removed → deleting the last project returns the
empty state) was verified against the Vite dev server with the same
mocked-`window.__TAURI_INTERNALS__.invoke` approach as TASK-005, extended
to also handle `update_project`. The disabled workspace tabs
(Processes/Captures/Documentation) were confirmed via
`element.disabled === true` — genuinely non-interactive, not just visually
muted. The native `golive.exe` was built and launched separately and
confirmed running via `tasklist` (killed after verification) against the
real database.

**Real-`AppData` update persistence:** unlike TASK-005's investigation
(which is not repeated here, per this task's explicit instruction), this
task's real-`AppData` spot-check was light and did **not** reproduce the
earlier anomaly — a project was created, then updated, then read back, as
three separate process invocations against `%APPDATA%\com.golive.app\`
(via a temporary `cargo run --example`, deleted before commit), and each
step returned exactly what the previous step had written
(`created_at` preserved, `updated_at` changed, name updated). This is
reported as an observation, not treated as confirmation the TASK-005
anomaly can't recur — per this task's instruction, no infrastructure
changes were made in response to either result.

**TASK-007 validation:** `npx tsc --noEmit`, `npm run build`, `cargo
check` (no warnings), `cargo test` (63/63 passing — 44 pre-existing + 19
new: 11 repository tests covering create/list-by-project/list-ordering/
update incl. id-project_id-created_at immutability/delete/not-found
variants/cascade-delete/reopen-persistence, plus 2 new `db` tests
confirming the `processes` table, its indexes, and its foreign key exist
after migration, and 15 service tests covering create incl. Draft
default/trimming/all four validation-rejection cases/missing-project,
update incl. status change/invalid-status rejection/timestamp
regeneration+preservation, and delete), and `npm run tauri build` all
pass.

Full UI-flow verification (Projects → create project → workspace opens →
Processes tab now enabled, Captures/Documentation confirmed still
`disabled` via `element.disabled === true` → empty state → create process
→ appears with "Draft" badge, auto-selected → detail shows name/
description/status/dates/reserved "Captures"+"AI analysis" cards → Edit:
values pre-filled incl. status select → Cancel discards (verified against
both UI and mock backend state) → Edit again, change status to "In
progress" → Save persists, badge/dates update immediately → create a
second process → edit the first again (status → "Completed") → confirms
it moves back to the top of the list → Delete requires confirmation →
deleting returns to the empty state → re-added a process, then deleted
the **Project** itself from the workspace → confirmed via mock state
(`window.__mockProjects`/`window.__mockProcesses`) that both the project
and its process were removed, proving the cascade → Settings still works)
was verified against the Vite dev server with the same mocked-IPC
approach as TASK-005/006, extended to cover all five Process commands
(with the mock strictly validating the `{ input: { project_id } }` /
`{ input: { project_id, name, description } }` shapes the frontend
service sends) and project-delete cascade. The native `golive.exe` was
built and launched separately (both before and after the temporary
real-`AppData` spot-check below) and confirmed running via `tasklist`
against the real, schema-upgraded (`user_version = 3`) database.

**Real-`AppData` Process persistence:** a light spot-check (a temporary
`cargo run --example`, deleted before commit) created a project and a
process, updated the process in a second process invocation, then listed
it in a third — each step returned exactly what the previous one had
written (`created_at` preserved, `updated_at` changed, status/name
updated). No anomaly reproduced this time either; per this task's
explicit instruction, this is reported as an observation only, and no
speculative infrastructure changes were made because of either this or
TASK-005's earlier finding.

**TASK-008 validation:** `npx tsc --noEmit`, `npm run build`, `cargo
check` (no warnings), `cargo test` (95/95 passing — 63 pre-existing + 32
new: 12 repository tests covering create/list-by-process/scoping/
ordering/get/update incl. id-process_id-created_at immutability/delete/
not-found variants/reopen-persistence/cascade-delete-from-process/
cascade-delete-through-process-from-project, plus 3 new `db` tests
confirming the `captures` table, its indexes, and its foreign key exist
after migration and one confirming the schema reaches `user_version = 4`,
and 16 service tests covering create incl. trimming/all four
validation-rejection cases/all three valid capture types/invalid capture
type/missing process, update incl. type change/invalid-type rejection/
timestamp regeneration+preservation, and delete), and `npm run tauri
build` all pass.

Full UI-flow verification (Projects → open project → Processes tab →
create process → select → Captures section shows the empty state → create
capture (Type defaults to Screenshot) → appears, auto-selected → detail
shows title/description/type/dates → Edit: values pre-filled → Cancel
discards (verified against both UI and mock backend state) → Edit again,
change title/type/description → Save persists, badge/dates update
immediately, capture stays first in the list → create a second capture,
confirm `updated_at DESC` ordering → Delete requires confirmation →
Cancel leaves the capture in place (verified against mock state) →
Delete again removes it and clears selection → delete the last capture →
empty state returns) was verified against the Vite dev server with the
same mocked-`window.__TAURI_INTERNALS__.invoke` approach as
TASK-005/006/007, extended to cover all five Capture commands (with the
mock strictly validating the `{ input: { process_id, capture_type, title,
description } }` / `{ input: { id, capture_type, title, description } }`
shapes the frontend service sends, and rejecting an `update_capture`
input that carries `process_id`/`created_at`/`updated_at`). Additional
checks against the mocked backend directly: an empty-title submit is
blocked client-side by HTML5 `required` before any `invoke()` call is
made; an over-length title/description and an invalid capture type are
all rejected with `AppError::Validation`; creating a capture against a
nonexistent process is rejected with `AppError::NotFound`; deleting a
Process cascades to remove its Captures; deleting a Project cascades
through its Processes to their Captures (confirmed via
`window.__mockProjects`/`__mockProcesses`/`__mockCaptures`); the Project
Workspace's "Captures" tab is still genuinely `disabled` (confirmed via
`element.disabled === true`); Settings continues to work. The native
`golive.exe` was built and launched separately and confirmed running via
`tasklist` (killed after verification) against the real, schema-upgraded
(`user_version = 4`) database.

**Real-`AppData` spot-check:** unlike TASK-005/006/007, no separate
multi-process real-`AppData` exercise was performed for the Capture
domain specifically — it wasn't required by this task's instructions, and
the automated `capture_survives_reopening_the_database` and
cascade-delete tests already exercise the identical `DbService`/
repository code path against a real (temp-directory) SQLite file across
separate `DbService` instances. The compiled binary was confirmed to
start and keep running cleanly against the real, migrated per-user
`%APPDATA%\com.golive.app\` database.

**TASK-009 validation:** `npx tsc --noEmit`, `npm run build`, `cargo
check` (no warnings), `cargo test` (117/117 passing — 95 pre-existing +
22 new: 11 `media::tests` covering directory creation/idempotence,
save/read round-trip, exists, filename-from-id, missing-file read/delete
handled gracefully, path-traversal rejected for several malicious ids,
reconcile removing only orphaned `.png` files, reconcile on a missing
directory; 1 new repository test (`list_all_ids`); 10 new service tests
covering metadata-only `create` never touching media, end-to-end
`create_screenshot`, missing-process/empty-title rejection, the two
transactional-cleanup cases (engine failure leaves nothing behind;
metadata-insert failure after a successful PNG write cleans the PNG back
up), a type-change preserving media, delete removing both metadata and
media, deleting a metadata-only capture with no media succeeding
gracefully, and cascade-orphan reconciliation), and `npm run tauri build`
all pass.

Full UI-flow verification (Captures section → "+ New capture" → Type
defaults to Screenshot, hint text and "Capture screenshot" button
confirmed → submit → capture created, auto-selected, `<img>` renders a
real decoded PNG via a `blob:` object URL fetched through
`get_capture_media` → Edit: change Type to Note → Save → preview
disappears from the UI but the mock backend's media map still holds the
bytes (confirmed via `window.__mockMedia`) → Edit again, change Type back
to Screenshot → Save → the *same* image reappears, proving media survives
a round-trip type change → Delete requires confirmation → confirmed
deletion removes both the Capture and its entry from
`window.__mockMedia`, and the empty state returns) was verified against
the Vite dev server with the same mocked-`window.__TAURI_INTERNALS__.invoke`
approach as prior tasks, extended with a `create_screenshot_capture`
handler (asserting the real command's exact input shape — no
`capture_type` field) and a `get_capture_media` handler returning
in-memory bytes for a real, valid 1×1 PNG.

**Native screenshot capture — manually verified, not simulated:**
- `native::screenshot::tests::capture_primary_display_smoke_test`
  (`#[ignore]`d — real display capture can't be assumed available in
  every environment `cargo test` runs in) was run explicitly
  (`cargo test --release -- --ignored capture_primary_display_smoke_test`)
  in this session's environment and **passed**: `WindowsScreenshotEngine`
  found one real monitor (`\\.\DISPLAY1`, 1920×1080, primary) and
  produced a valid PNG.
- A further one-off spot-check (`examples/screenshot_spotcheck.rs`,
  written temporarily and deleted before commit — same pattern as
  TASK-004 through TASK-008's real-environment checks) wrote the captured
  PNG to disk and it was opened and visually inspected: it is a genuine,
  correct screenshot of this session's real Windows desktop (not a
  black/blank frame, not a placeholder) — content specific to the actual
  desktop was visible in the captured image, satisfying this task's
  requirement to confirm real desktop content, not just a non-empty byte
  count.
- A second temporary example (`examples/appdata_spotcheck.rs`, also
  deleted before commit; `lib.rs`'s modules were briefly widened to
  `pub` to let it reach `CaptureService`/`MediaStorage` directly, then
  reverted immediately after — same temporary-widening approach implied
  by TASK-004 through TASK-008's real-`AppData` checks) exercised the
  full **production** pipeline as three separate process invocations
  against the real `%APPDATA%\com.golive.app\` directory:
  1. `create`: `CaptureService::create_screenshot` captured the real
     display and wrote `captures\<id>.png` (612,536 bytes) alongside a
     real metadata row — confirmed present on disk via a direct
     `AppData` directory listing.
  2. `verify` (a fresh process — simulates closing and reopening
     GoLive): re-initialized `DbService`/`MediaStorage` from scratch and
     confirmed both the Capture metadata and the exact same media bytes
     were still readable — **screenshot persistence across restart,
     proven against the real database and real filesystem, not a temp
     directory.**
  3. `delete` (a third fresh process): called `CaptureService::delete`
     and confirmed both `get`/`get_screenshot_media` now report
     `NotFound`, and confirmed via a direct directory listing that the
     PNG file was physically removed from
     `%APPDATA%\com.golive.app\captures\` — **media deletion, verified
     against the real filesystem.**
  
  Both temporary example files and the temporary `pub` widening in
  `lib.rs` were removed before this task was considered complete; `git
  status` was checked afterward to confirm no stray files remained.
- What was **not** performed: clicking through the compiled `golive.exe`'s
  actual native window UI end-to-end (Projects → Process → Captures →
  "Capture screenshot" button → preview → Delete) — no desktop
  UI-automation tool is available in this environment to drive the native
  WebView2 window directly (the Browser pane drives a real web browser
  tab, not the Tauri window), the same limitation TASK-003/004/005 also
  recorded. Given the above — the real capture engine verified against
  real desktop content, and the full real production
  `CaptureService`/`MediaStorage` pipeline verified against the real
  `%APPDATA%` directory across separate process "restarts" — this is
  judged sufficient evidence that the feature works correctly end to end
  without overclaiming a native-UI click-through that could not actually
  be performed here. A developer with an interactive desktop session
  should still do one unassisted pass through `golive.exe`'s UI per this
  task's own §33 checklist before relying on this in a real engagement.

## Not implemented yet

- Recording/Note capture media (TASK-009 gave only `screenshot` real
  media; Note and Recording Captures remain metadata only, with no
  actual file captured, stored, or attached)
- Monitor selection, area/window-selection screenshots (TASK-009 only
  supports "capture the primary/current display")
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
  incremental proof-of-concept development). Single-frame primary-display
  screenshot capture (TASK-009) is implemented and manually verified
  (real display content, real `%APPDATA%` persistence, real deletion —
  see below), which de-risks the underlying native-capture library choice
  (`xcap`) for the recording work ahead, but recording itself (continuous
  capture, encoding, file size/performance) is unstarted and remains the
  highest-risk item
- Microphone capture and audio/video synchronization
- Native screen capture across multi-monitor setups (TASK-009
  deliberately supports only the primary/current display; monitor
  selection is unstarted)
- AI integration reliability (structured/schema-constrained output)
- Word document generation quality for a consulting-grade deliverable

## Next task

TASK-010
