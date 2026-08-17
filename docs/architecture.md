# Architecture

This document describes GoLive's architecture: what exists today (**CURRENT**)
and the conventions future tasks must follow as functionality is added
(**FUTURE**). It is updated as the implementation evolves — see
[PROJECT_STATE.md](../PROJECT_STATE.md) for the current milestone and
[DECISIONS.md](../DECISIONS.md) for the reasoning behind each choice.

## 1. Architecture overview

**CURRENT**

- **Desktop shell:** Tauri 2
- **Frontend:** React + TypeScript (Vite)
- **Backend/native layer:** Rust
- **Target platform:** Windows desktop only (MVP)

```
React (frontend)
  ↓ invoke()
Tauri command   (src-tauri/src, thin — translates a frontend call
  ↓              into an application-service call and back)
Rust service     [FUTURE — introduced with first real business logic]
  ↓
Repository       [FUTURE — introduced with SQLite]
  ↓
SQLite           [FUTURE]
```

React never accesses the filesystem, database, or native OS APIs directly.
All such access goes through a Tauri command. This keeps business logic
testable independently of the UI and keeps a future cloud/server backend
possible without rewriting the frontend (see §14).

## 2. Current frontend structure

```
src/
  components/   reusable, presentation-only UI pieces
  features/     feature-scoped modules (projects, captures, processes, ...)
  pages/        top-level routed views composing features
  services/     app-level wrappers around Tauri invoke() calls
  stores/       cross-feature client-side state
  types/        shared TypeScript domain types
  utils/        small pure helpers
```

Conventions per folder:

- **`components/`** — Reusable UI building blocks (buttons, inputs, cards,
  layout primitives). Presentation only; minimal-to-no domain/business
  logic. Not tied to a specific feature.
- **`features/`** — Domain-specific functionality (future: `projects/`,
  `captures/`, `recordings/`, `screenshots/`, `processes/`, `ai/`,
  `settings/`). **Start simple, split only when complexity justifies it**: a
  new feature begins as a single file or small flat folder; only introduce
  `components/`, `services/`, `hooks/`, `types/`, `tests/` subfolders inside
  it once it genuinely has enough of each to need the separation. Don't
  pre-scaffold empty feature subfolders.
- **`pages/`** — Top-level routed views that compose features and
  components into full screens. Pages coordinate; they don't own business
  logic.
- **`services/`** (top level) — App-level Tauri `invoke()` wrappers that
  don't belong to one feature (see §4). Feature-specific service calls
  belong in `features/<feature>/services/` instead once that feature
  exists.
- **`stores/`** — Shared client-side state that genuinely spans multiple
  features or pages (see §11). Currently empty — nothing needs it yet.
- **`types/`** — Shared TypeScript types for domain entities used across
  features (Project, Process, Capture, ...). Currently empty — no domain
  entities exist yet (see §5).
- **`utils/`** — Small, pure, framework-agnostic helpers with no domain
  ownership. Currently empty.

**Current contents:** `App.tsx` composes the application shell (see §16).
`components/layout/` holds `AppShell`, `Sidebar`, `Header`; `components/ui/`
holds the generic pieces reused across the app so far — `EmptyState`,
`StatusPill`, `Dialog`. `pages/ProjectsPage.tsx` is now a thin wrapper
around the real `features/projects/` feature (see §18);
`pages/SettingsPage.tsx` is still a placeholder. `types/` holds
`navigation.ts`. `services/` holds the app-level Tauri wrappers
(`foundation.ts`, `storage.ts`); `features/projects/services/projects.ts`
holds the feature-specific ones. `utils/` holds `formatDate.ts` and
`errorMessage.ts`. `stores/` remains empty — nothing needs it yet (see
§11).

## 3. Current Rust/Tauri structure

**CURRENT:**

```
src-tauri/
  migrations/
    0001_initial.sql        infrastructure-only migration (see §17)
    0002_projects.sql        Project domain schema (see §18)
  src/
    main.rs                 entry point
    lib.rs                  Tauri builder, .setup() hook, command registration
    errors.rs                AppError (see §10)
    commands/
      foundation.rs           check_foundation_status
      storage.rs               get_local_storage_status
      project.rs                create/list/get/delete_project (see §18)
    db/
      mod.rs                   DbService: init, pool access
      pool.rs                  r2d2 pool construction + PRAGMAs
      migrations.rs            migration runner
    models/
      project.rs                Project struct (see §18)
    repositories/
      storage_status.rs        StorageStatusRepository trait +
                                SqliteStorageStatusRepository
      project.rs                 ProjectRepository trait +
                                  SqliteProjectRepository (see §18)
    services/
      project.rs                 ProjectService: validation, id/timestamp
                                  generation (see §18)
```

`commands/`, `db/`, and `repositories/` were introduced by TASK-004.
`models/` and `services/` were introduced by TASK-005, once the Project
domain gave them real content — `db::DbService` was always
infrastructure, not domain logic, so it didn't count as a reason to add
`services/` on its own; `ProjectService` is the first actual occupant.

## 4. Frontend → Tauri communication

**Convention (CURRENT, established by this task):**

```
React component
    ↓
frontend service function      src/services/*.ts
    ↓                          or features/<feature>/services/*.ts
Tauri invoke()
    ↓
Tauri command                  src-tauri/src/commands/*.rs
    ↓
application service            [FUTURE — no business logic exists yet]
    ↓
repository / native service     src-tauri/src/repositories/*.rs (CURRENT
                                 for storage; native service still FUTURE)
```

**Rule:** React components never call `invoke()` directly. They call a
service function, which is the only place `invoke()` appears.

**Where the service function lives:**
- Not tied to a specific feature → `src/services/`.
- Belongs to one feature → `features/<feature>/services/`.

**Concrete examples:**
- App-level (no feature): `App.tsx` calls `checkFoundationStatus()` from
  [`src/services/foundation.ts`](../src/services/foundation.ts) →
  `invoke("check_foundation_status")` →
  [`commands::foundation::check_foundation_status`](../src-tauri/src/commands/foundation.rs).
  Infallible, no persistence — no application-service/repository step was
  added around it, since none would do anything.
- App-level: `SettingsPage` calls `getLocalStorageStatus()` from
  [`src/services/storage.ts`](../src/services/storage.ts) →
  `invoke("get_local_storage_status")` →
  [`commands::storage::get_local_storage_status`](../src-tauri/src/commands/storage.rs),
  which goes through `SqliteStorageStatusRepository` (§17) — fallible
  (`Result<T, AppError>`, §10) and reaches a repository.
- Feature-specific (TASK-005, the first one): `ProjectsView` calls
  `createProject()`/`listProjects()`/etc. from
  [`features/projects/services/projects.ts`](../src/features/projects/services/projects.ts)
  → `invoke("create_project")` etc. →
  [`commands::project`](../src-tauri/src/commands/project.rs) →
  `ProjectService` → `ProjectRepository` — the full chain §18 documents
  in detail, and the reason the `features/<feature>/services/` half of
  this rule exists at all.

## 5. Business logic boundary

**Rule:** business rules do not live in React components (or in Tauri
command handlers, once those exist beyond trivial delegation). They live in
Rust application services (see §3), independently of the UI.

**CURRENT, first real occupant (TASK-005):** `ProjectService::create`
trims and validates `name`/`description` and generates the id/timestamps
— this logic lives there, not in `commands::project` (which just
delegates) and not in the frontend (which sends raw, untrimmed input and
lets the backend be the source of truth). See §18.

Examples of rules that will apply this boundary later, not implemented
yet:
- "A Capture belongs to zero or one Process."
- "AI-generated process regeneration must not silently overwrite a
  previous process version."

A React component (or a Tauri command) should call a service function and
render/relay the result — it should not itself decide what the rule is.

## 6. Persistence boundary

**CURRENT.** SQLite persistence infrastructure (TASK-004) now has its
first real domain occupant: the Project entity (TASK-005). See §17 for
the infrastructure and §18 for the Project-specific detail.

```
Application/domain logic (Rust)     CURRENT — ProjectService (§18)
        ↓
Repository interface (Rust trait)   CURRENT — StorageStatusRepository,
        ↓                                     ProjectRepository
SQLite implementation               CURRENT — SqliteStorageStatusRepository,
                                                SqliteProjectRepository
```

The frontend never knows SQLite exists — it only ever calls a Tauri
command. A future cloud architecture should be able to replace the SQLite
repository implementation with a remote API/server repository behind the
same trait, without changing the command layer or the UI.

## 7. File storage boundary

**FUTURE.** GoLive will eventually store recordings, screenshots, audio,
project files, and exported documents on disk.

**Rule:** native filesystem operations live in the Rust/native layer
(behind a future storage service, analogous to the repository boundary in
§6), never directly in React. No storage code is implemented by this task.

## 8. AI boundary

**FUTURE.** AI is cloud-only (OpenAI initially).

```
Process/AI feature (Rust + frontend)
       ↓
AI service abstraction (Rust trait)
       ↓
AI provider implementation (OpenAI today)
       ↓
OpenAI API
```

The rest of the application depends only on the AI service abstraction,
never on OpenAI-specific types or endpoints directly, so another provider
can be added later without touching callers. No AI code or dependency is
introduced by this task.

## 9. Native Windows functionality boundary

**FUTURE.** Screen recording, screenshots, microphone capture, and global
shortcuts require native Windows APIs.

**Rule:** this functionality is implemented as Rust/Tauri services exposed
through commands. React consumes a clean application API (e.g. "start
recording", "take screenshot") — it never manipulates Windows APIs
directly. Nothing under this boundary is implemented yet.

## 10. Error handling

**CURRENT**, implemented TASK-004 (`src-tauri/src/errors.rs`), now that
the database layer introduced the project's first genuinely fallible
commands.

- `AppError` (`thiserror`-derived) has one variant per failure category
  the app currently has: `Storage` (app-data directory / connection
  issues), `Database` (query/repository failures), `Migration` (schema
  migration failures), `Validation(String)` (TASK-005 — an
  author-written, safe, specific message like "Project name is
  required.", the one variant whose message *is* shown to the user
  as-is, since it's never a raw underlying error), `NotFound` (TASK-005 —
  `get`/`delete` by an id that doesn't exist). Every other variant
  carries a fixed, generic, user-safe message — never a raw underlying
  error.
- `AppError` implements `serde::Serialize` by hand, always producing
  `{ "code": "...", "message": "..." }` for the frontend — the code is a
  stable machine-readable string (e.g. `"database_error"`), the message is
  the fixed generic text, never SQL text, file paths, or other internals.
- `impl From<std::io::Error>`, `From<r2d2::Error>`, `From<rusqlite::Error>`
  for `AppError` log the real underlying error to stderr (`eprintln!`,
  prefixed `[golive]`) at the point of conversion, then return the generic
  variant — so the detail needed to diagnose a failure stays available to
  a developer without ever reaching the UI or the frontend.
- Fallible Tauri commands return `Result<T, AppError>` — see
  `commands::storage::get_local_storage_status`. No expected failure path
  uses `panic!`/`unwrap`/`expect`; `.expect()` remains reserved for
  genuinely unrecoverable startup failures (`run(...).expect(...)` in
  `lib.rs`, and database initialization failing inside `.setup()`, which
  intentionally prevents the app from starting rather than running with a
  broken database silently — see §17).
- The frontend service layer (§4) receives the rejected `invoke()` promise
  and turns it into a plain-language, non-technical status (see
  `SettingsPage`'s "Local storage: Unavailable"), keeping the structured
  `{ code, message }` available (currently as a hover tooltip) for
  debugging rather than showing it as primary UI text.

An `AppError::State` variant (for "invalid application state") was
considered, per the general category this task was asked to cover, but
deliberately not added: nothing in the current code can actually produce
it (Tauri's own `State<T>` extractor handles a truly unmanaged state as a
framework-level error, not one our code constructs), and an unused variant
would just be dead code. It can be added the moment a real path to it
exists.

## 11. State management

**CURRENT:** `App.tsx` still uses local component state (`useState`) for
connectivity status. The Project feature owns its own state, all in
`features/projects/ProjectsView.tsx`: the project list, loading/error
status, which dialog (if any) is open, and — the ongoing example of
"feature state" — **`activeProject`** (TASK-005 called this `selectedId`;
TASK-006 renamed/promoted it to hold the full `Project`, since the
workspace needs the whole record, not just an id to look up). No state
management library is installed.

**Rule:**
- **Local UI state** (modal open/closed, form fields, a single component's
  status) → local component state (`useState`/`useReducer`). Default
  choice. Example: `CreateProjectDialog`/`EditProjectDialog`'s form fields
  and submitting flag.
- **Feature state** (e.g. active project, future active-recording state,
  process-editor state) → owned within that feature, introduced only once
  the feature needs it. Example: `activeProject` in `ProjectsView` —
  deliberately just `useState<Project | null>`, so if a future feature
  (e.g. a floating widget) needs to know the current project too, this is
  a same-shape swap into a `stores/` slice, not a rewrite.
- **Application state** (e.g. future global settings) → `src/stores/`,
  introduced only once at least two features genuinely need to share it.
  Still empty — active-project state doesn't qualify yet, since only the
  Projects feature reads it.

No state management library is adopted by this task. If/when `stores/`
state is actually needed, the simplest option that fits (React context, or
a minimal library) is chosen at that time — not decided speculatively now.

## 12. Configuration and secrets

- **Source-controlled configuration:** `tauri.conf.json`, `package.json`,
  `Cargo.toml`. Never contains secrets.
- **Runtime/project configuration:** future per-project settings (client,
  content language, etc.) — local to the user's machine, not secret, not
  implemented yet.
- **Secrets:** the future OpenAI API key is stored exclusively in the
  **Windows Credential Manager** — never in source code, Git, SQLite, or
  plain JSON/config files. Not implemented by this task; no placeholder or
  fake key exists anywhere in the repo.
- `.gitignore` explicitly excludes `.env`/`.env.*` (keeping `.env.example`
  allowed) as a defensive guard, even though no `.env`-based configuration
  is in use yet.

## 13. Tauri security / capabilities

**CURRENT:** [`src-tauri/capabilities/default.json`](../src-tauri/capabilities/default.json)
grants exactly one permission: `core:default`. No filesystem, shell,
global-shortcut, microphone, screen-capture, HTTP, or process-control
permissions are requested.

**Rule:** each future capability (filesystem access for project storage,
global shortcuts, microphone, screen capture, HTTP for the AI provider,
etc.) is added only in the task that implements the corresponding
functionality, scoped as narrowly as Tauri allows for that feature. No
capability is granted ahead of the code that needs it. The Project
commands (TASK-005) needed none of these — they only touch the already-
managed SQLite connection — so capabilities are unchanged.

CSP is currently `null` (framework default). It will be tightened once the
app's actual resource-loading needs (fonts, local media, any remote calls)
are known, rather than guessed at now.

## 14. Future cloud migration principle

The repository (§6), storage (§7), and AI provider (§8) boundaries all
exist for the same reason: each one lets a local-only implementation
(SQLite, local filesystem, OpenAI) be swapped for a remote/cloud
counterpart later by replacing what sits behind the abstraction — without
rewriting Tauri commands or the React UI. This is a design constraint to
keep in mind, not something implemented yet.

## 15. Dependency philosophy & inventory

**Philosophy:** prefer mature, actively maintained, minimal dependencies.
Add a dependency only when a task genuinely needs it; don't remove a
working one just to shrink the list.

**Current npm dependencies:**

| Package | Purpose |
|---|---|
| `react`, `react-dom` | UI framework |
| `@tauri-apps/api` | `invoke()` and other frontend↔Tauri bindings |
| `typescript` | Type checking |
| `vite`, `@vitejs/plugin-react` | Dev server / build tooling |
| `@tauri-apps/cli` | `tauri dev` / `tauri build` commands |
| `@types/react`, `@types/react-dom` | Type definitions |

**Current Rust dependencies (`src-tauri/Cargo.toml`):**

| Crate | Purpose |
|---|---|
| `tauri` | Desktop shell / command runtime |
| `tauri-build` | Build-time codegen (build.rs) |
| `serde` (derive) | (De)serialization — now actually used: `LocalStorageStatus`, `AppError`. |
| `serde_json` | JSON value handling — still unused directly by our code; kept for the same reason as before (near-certain need once AI/JSON payloads exist), see TASK-002 decision. |
| `rusqlite` (`bundled`) | Embedded SQLite bindings. `bundled` statically compiles SQLite into the binary — no system SQLite install required (see §17, DECISIONS.md). |
| `r2d2`, `r2d2_sqlite` (`bundled`) | Connection pool for `rusqlite`, so one long-running database operation doesn't block every other one (see §17, "Concurrency"). |
| `thiserror` | Derives `AppError`'s `Display`/`std::error::Error` impl (§10) with minimal boilerplate. |
| `uuid` (`v4`) | Generates `Project` ids (§18). Already an indirect dependency of the toolchain; added as a direct one now that our own code (`ProjectService`) actually calls it. |

**Dev-only:** `tempfile` — isolated temp directories for the database
tests (§17), never touching the real per-user app-data directory.

Rejected for this task: `rusqlite_migration` (evaluated, added, then
removed — see DECISIONS.md for why) and `sqlx` (evaluated and not chosen
— see DECISIONS.md). `tauri-plugin-opener` remains removed from TASK-001.

## 16. Application shell & navigation

**CURRENT**, established by TASK-003.

- **Layout components** (`src/components/layout/`): `AppShell` (structural
  frame — sidebar / header / scrollable content region), `Sidebar`
  (persistent left navigation), `Header` (active area title + a
  non-technical connectivity indicator). Each is presentation-only and
  owns no navigation or connectivity state itself — that's passed in as
  props by `App.tsx`.
- **Generic reusable UI** (`src/components/ui/`): `EmptyState`,
  `StatusPill`, `Dialog` (the latter two added in TASK-004/005 — see §17,
  §18). New components go here only once actually reused (see §2's
  `components/` convention).
- **Pages** (`src/pages/`): `SettingsPage.tsx` is still an empty-state
  placeholder; `ProjectsPage.tsx` is now a thin wrapper around the real
  `features/projects/` feature (§18) — pages stay free of business logic
  either way.
- **Navigation:** `App.tsx` owns `activeView` (`useState<AppView>`,
  `AppView`/`NavItem` defined in `src/types/navigation.ts`) and passes it
  and a setter down to `Sidebar` and `Header` as props — no store needed
  since only `App.tsx` and its two direct children touch it. Which page
  renders for which view is a small `Record<AppView, () => ReactNode>` map
  in `App.tsx`.
- **Why no routing library:** with exactly two views and no deep-linking
  or browser back/forward requirement yet, `react-router` (or similar)
  would add a dependency and API surface with nothing to justify it (see
  DECISIONS.md). The `Record<AppView, ...>` map is deliberately shaped so
  swapping it for a router later is mechanical:
  - each `PAGES` entry becomes a `<Route path="/..." element={<... />} />`;
  - `activeView` / `setActiveView` are replaced by the router's own
    location state / `navigate()`;
  - `Sidebar`'s `onNavigate` callback becomes a `<Link>` or `navigate()`
    call.
  - `AppShell`, `Sidebar`, and `Header` need no changes either way, since
    none of them depends on how navigation state is implemented.
- **Styling:** `src/styles/tokens.css` defines the design tokens (color,
  spacing, radius, font — including a `prefers-color-scheme: dark`
  variant, continuing what TASK-001's scaffold already did). `src/App.css`
  holds the shell/page CSS built on those tokens. No CSS framework or
  CSS-in-JS dependency was introduced.
- **Window sizing:** `tauri.conf.json` now sets `minWidth`/`minHeight`
  (760×480) so the sidebar and content stay usable if the user shrinks the
  window.
- **Connectivity status:** unchanged mechanism — `App.tsx` still calls
  `checkFoundationStatus()` from `src/services/foundation.ts` on mount.
  The result now renders as a small status dot + label in `Header`
  ("Ready" / "Connecting…" / "Offline") instead of the old standalone
  card; the raw backend message is kept only as a hover tooltip, not
  shown in the main UI, per the "no technical details in normal user
  flow" rule. `Header`'s status indicator was extracted into the
  reusable `components/ui/StatusPill` in TASK-004, once `SettingsPage`
  needed the identical dot+label pattern for local storage status.

## 17. Database architecture (TASK-004)

**CURRENT.** Local SQLite persistence infrastructure — no domain schema
yet (that's TASK-005+).

**Technology:** [`rusqlite`](https://docs.rs/rusqlite) (`bundled`
feature — SQLite is compiled into the binary; the user never installs
SQLite separately), pooled via
[`r2d2`](https://docs.rs/r2d2)/[`r2d2_sqlite`](https://docs.rs/r2d2_sqlite).
`sqlx` was evaluated and rejected — see DECISIONS.md.

**Location:** `<app_data_dir>/database/golive.db`, where `app_data_dir`
comes from Tauri's `app.path().app_data_dir()` (resolved once, in
`.setup()`) — never hardcoded, never `src/`, never next to the executable.
The `database` subdirectory is created automatically
(`std::fs::create_dir_all`) if it doesn't exist.

**Initialization** (`db::DbService::init`, called once from `lib.rs`'s
`.setup()` hook): resolve `app_data_dir` → ensure `database/` exists →
open/create `golive.db` → build the connection pool (applying pragmas on
every new pooled connection, see below) → run pending migrations → return
a `DbService` that owns the pool. Idempotent — calling it again against an
existing database neither recreates nor destroys it (see `db::tests`). If
any step fails, `.setup()` returns `Err`, and Tauri does not start the
application — an initialization failure is never silently ignored.

**SQLite configuration**, applied to every pooled connection on creation:
- `foreign_keys = ON` (off by default in SQLite; needed once real tables
  reference each other).
- `journal_mode = WAL` — lets readers proceed while a write is in
  progress, rather than the stricter default rollback-journal locking.
- `busy_timeout = 5000` — a connection that finds the database briefly
  locked waits up to 5s instead of failing immediately.

**Migrations:** hand-rolled rather than a third-party crate (see
DECISIONS.md for why) — `db::migrations::run` reads SQLite's
`user_version` pragma, then applies, in order and each inside its own
transaction, every file in a fixed `MIGRATIONS: &[(version, sql)]` array
whose version is greater than the current one, bumping `user_version`
after each. Files live in `src-tauri/migrations/` (`0001_initial.sql` today, which
deliberately contains only an infrastructure table, no domain schema —
see DECISIONS.md), are embedded at compile time (`include_str!`), and
must never be edited once shipped — only appended to. Applying the same
migrations twice is a no-op, so this runs safely on every startup.

**Repository boundary:** `repositories::storage_status::StorageStatusRepository`
(trait, one method: `ensure_marker`) +
`SqliteStorageStatusRepository` (the only implementation). Deliberately
not a generic `Repository<T>` — there's no real domain model yet to
justify one; this is the concrete instantiation of the pattern §6
describes, scoped to exactly the proof-of-persistence operation. Callers
(Tauri commands) depend on the trait, not the SQLite type.

**Connection management:** commands don't open connections or manage
pools themselves — they receive the pool via `db::DbService`
(Tauri-managed state, `app.manage(db_service)`), and construct a
repository from it per call
(`SqliteStorageStatusRepository::new(db.pool())` — cloning an `r2d2::Pool`
is cheap, it's `Arc`-backed). No database initialization logic lives
inside a command (see `commands::storage::get_local_storage_status`).

**Error handling:** see §10. Every fallible database operation returns
`Result<_, AppError>`; raw `rusqlite`/`r2d2`/`io` errors are logged to
stderr and converted to a generic `AppError::Storage` /
`AppError::Database` / `AppError::Migration` before reaching a command's
return value — never exposed to the frontend.

**Testing strategy:** all database tests (`db::tests`,
`repositories::storage_status::tests`) use `tempfile::tempdir()` — a
fresh, isolated directory per test, deleted automatically when the test
ends — never the real per-user `app_data_dir`. `cargo test` covers:
database/directory creation, `init` being safe to call repeatedly
(simulating repeated app launches), the migrated schema being queryable,
`ensure_marker` writing once and returning the same value on repeated
calls, and — the most important one —
`marker_survives_reopening_the_database`, which drops the `DbService`
(closing the pool, simulating closing the app) and re-initializes a new
one against the same directory (simulating relaunching it), asserting the
marker read back matches what was written. This was additionally verified
manually against the real per-user `app_data_dir` (see PROJECT_STATE.md).

**Proof-of-persistence:** `get_local_storage_status` (command) →
`SqliteStorageStatusRepository::ensure_marker` — writes a fixed marker key
to `app_metadata` exactly once (first-ever call), reads it back on every
call after. `SettingsPage` calls this on mount and renders "Local storage:
Ready" (no path, no SQL, no raw error) — see §4 for the full call chain.

**Future cloud replacement:** unchanged principle from §6/§14 —
`StorageStatusRepository` (and whatever repositories TASK-005+ add) is
the seam a remote implementation would sit behind; `DbService` and
`SqliteStorageStatusRepository` are the only things that would be
replaced, not the commands or the frontend.

## 18. Project domain (TASK-005)

**CURRENT.** The first real GoLive domain entity — the top-level
container future Processes/Captures/Screenshots/Recordings/generated
documentation will belong to (via a `project_id` foreign key, once those
tables exist — not yet).

**Model** (`src-tauri/src/models/project.rs`): `Project { id, name,
description, created_at, updated_at }`.
- `id`: a `Uuid::new_v4()` string, generated by `ProjectService` — never
  accepted from the frontend. The frontend never generates database IDs.
- `created_at`/`updated_at`: Unix epoch **milliseconds**, UTC, generated
  by the backend — never accepted from the frontend. `created_at` is set
  once and never changes; `updated_at` is regenerated on every successful
  `update` (TASK-006 — see §19).

**Full flow (the concrete instantiation of the pattern §4–§6 describe):**

```
Projects UI (features/projects/)
    ↓
frontend project service     features/projects/services/projects.ts
    ↓
Tauri invoke()
    ↓
Tauri command                 commands::project (create/list/get/update/delete_project)
    ↓
ProjectService                 services::project — validation, id/timestamp
    ↓                                              generation, business rules
ProjectRepository (trait)      repositories::project
    ↓
SqliteProjectRepository        all Project SQL lives here, nowhere else
    ↓
SQLite (projects table)
```

**Repository** (`repositories/project.rs`): `ProjectRepository` trait —
`create`, `list` (ordered `updated_at DESC`), `get`, `update` (TASK-006),
`delete` — `update`/`delete` both return whether a row actually existed,
so the service can distinguish "changed" from "not found" — plus
`SqliteProjectRepository`, the only implementation.

**Service** (`services/project.rs`): `ProjectService::create`/`update`
both trim `name`/`description`, reject an empty (post-trim) name, and
enforce length limits (`name` ≤ 200, `description` ≤ 5000 Unicode scalar
values — not bytes, since project content language is configurable per
project and a 200-character Italian name shouldn't be rejected sooner
than an English one just because it has more accented letters).
`update` additionally verifies the project exists (via `get`, which maps
a missing row to `AppError::NotFound`) before writing, and regenerates
`updated_at` while carrying `id`/`created_at` forward from the existing
record — the caller's input has no way to influence either.

**Commands** (`commands/project.rs`): `create_project`, `list_projects`,
`get_project`, `update_project`, `delete_project` — each just builds a
`ProjectService` from the managed `DbService` and delegates. No SQL, no
validation logic, no business rules in the command layer (same pattern as
`commands::storage`). `update_project` takes an explicit
`UpdateProjectInput { id, name, description }` — not the `Project` model
itself — so a request has no field for `created_at`/`updated_at` to
occupy, accidentally or otherwise.

**Frontend service** (`features/projects/services/projects.ts`): typed
`createProject`/`listProjects`/`getProject`/`updateProject`/
`deleteProject`, mapping the wire-format `RawProject` (`snake_case`,
matching the Rust struct exactly) to a `Project` type (`camelCase`) — the
same manual-mapping convention `services/storage.ts` already established,
kept consistent rather than introducing
`#[serde(rename_all = "camelCase")]` as a second convention.

**Frontend feature** (`features/projects/`) — the first feature to
actually need the `components/`/`services/` split §2 describes. See §19
for the Workspace/Overview/editing pieces added in TASK-006.
- `ProjectsView.tsx` — feature root, composed by the thin
  `pages/ProjectsPage.tsx`. Owns list/loading/error state, which dialog
  is open, and `activeProject` (see §11).
- `components/ProjectList.tsx`, `CreateProjectDialog.tsx`,
  `DeleteProjectDialog.tsx`, plus `ProjectWorkspace.tsx`,
  `ProjectOverview.tsx`, `EditProjectDialog.tsx` (§19).
- Reuses `components/ui/EmptyState` (no projects / list failed to load)
  and the shared `components/ui/Dialog` (generic modal shell —
  Escape/backdrop-click to close, `role="dialog"`, used by all three
  project dialogs; not feature-specific, so it lives in the app-level
  `components/ui/`, not inside `features/projects/`).

**Validation surfaces twice, deliberately:** the `<input required
maxLength={200}>` in `CreateProjectDialog` gives instant feedback and
blocks obviously-invalid submits client-side, but the backend
(`ProjectService`) is the actual source of truth — it re-validates
independently (trimming, length, emptiness) and is what every test in
`services::project::tests` exercises. The frontend never trusts its own
validation as sufficient.

**Errors:** `AppError::Validation` (empty name, over-length) and
`AppError::NotFound` (get/update/delete a missing project) — see §10.
Displayed via `getErrorMessage()` (`utils/errorMessage.ts`), which
extracts the safe `{ message }` from a rejected `invoke()` call; used by
all three dialogs and the list's error state. `isNotFoundError()` (same
file) lets a caller react specifically to a `not_found` code — used by
`EditProjectDialog` to exit the workspace gracefully instead of showing a
generic error if the project was deleted elsewhere mid-edit (§19), and by
`DeleteProjectDialog` to treat "already gone" the same as "successfully
deleted."

**Loading/error states:** `ProjectsView` shows "Loading projects…" while
the initial `listProjects()` call is in flight, an error state with Retry
if it fails, an empty state if it succeeds with zero projects, or the
project list otherwise (selecting a project switches to the Workspace,
§19). `CreateProjectDialog`/`EditProjectDialog`/`DeleteProjectDialog` each
disable their submit button and relabel it while their own request is in
flight ("Creating…"/"Saving…"/"Deleting…"), and refuse to resubmit — a
second click while the request is in flight is a no-op.

**Testing:** see §19 for the current test count and TASK-006's UI-flow
verification (which folds in and re-verifies everything described here).
No frontend test framework exists in this repo and none has been
introduced for the Project domain (per each task's own instruction) — UI
flows are verified interactively against the Vite dev server with a
mocked Tauri IPC bridge, then the compiled native app is launched
separately to confirm it starts cleanly against the real database. See
PROJECT_STATE.md for the full verification record, including a
noteworthy environment-specific finding (TASK-005) around real-`AppData`
persistence checks.

## 19. Project Workspace and editing (TASK-006)

**CURRENT.** Turns a selected Project into a real workspace, and adds
update (edit) end to end.

```
Projects (list)
    ↓ select a project
activeProject                  ProjectsView's useState<Project | null>
    ↓
Project Workspace              components/ProjectWorkspace.tsx
    ├── Overview                 IMPLEMENTED — components/ProjectOverview.tsx
    ├── Processes                NOT AVAILABLE YET (disabled tab)
    ├── Captures                 NOT AVAILABLE YET (disabled tab)
    └── Documentation            NOT AVAILABLE YET (disabled tab)
```

**Projects vs. Workspace — two distinct concepts, not two routes:**
`ProjectsView` renders one or the other based on `activeProject`, still
without a routing library (see the original reasoning in §16 — nothing
here changes that calculus: still no deep-linking/back-button requirement,
still one component deciding what to render). Selecting a project in the
list sets `activeProject`; the Workspace's "← Projects" button clears it.
No project data is destroyed by this transition — `activeProject` is just
a reference to an entry already held in `ProjectsView`'s `projects` array.

**Update flow (mirrors create/list/get/delete exactly, §18):**

```
Projects UI
    ↓
frontend project service    features/projects/services/projects.ts (updateProject)
    ↓
Tauri invoke("update_project")
    ↓
commands::update_project     thin — builds ProjectService, delegates
    ↓
ProjectService::update        validate/trim, verify existence, regenerate
    ↓                          updated_at, carry id/created_at forward
ProjectRepository::update      UPDATE ... SET name, description, updated_at
    ↓                          WHERE id = ? — id/created_at not in SET clause
SQLite (projects table)
```

**Workspace shell** (`ProjectWorkspace.tsx`): back button, project
name/description header (shown regardless of which tab is active — see
below), Edit/Delete actions, and the tab bar. Tabs are a small
`{id, label}[]` array, structurally identical to the top-level
`NAV_ITEMS`/`PAGES` pattern in `App.tsx` (§16) — the same seam a future
router would replace, just one level deeper (`/projects/:id/overview`
etc.). Only `AVAILABLE_TAB = "overview"` is enabled; the other three are
real `disabled` buttons with a "Not available yet" tooltip — genuinely
non-interactive, not fake-clickable.

**Header title decision:** the app-level `Header` (§16) still shows
"Projects" while inside the workspace — it was **not** changed to the
project name. The task explicitly allowed this ("if useful"); doing so
would have required lifting `activeProject` up into `App.tsx`, breaking
the feature's self-contained state (§11) for a cosmetic win the
workspace's own prominent name heading already delivers. See DECISIONS.md.

**Overview** (`ProjectOverview.tsx`): created/updated dates plus three
informational "reserved" cards (Processes/Captures/Documentation, each
with a one-line explanation of what will eventually live there) — the
same non-interactive placeholder pattern as the disabled tabs, reusing
generic `.reserved-section` CSS rather than the tab-specific classes.
Name/description are not repeated here — they're already shown once, in
the workspace header, visible regardless of tab.

**Edit** (`EditProjectDialog.tsx`): a third dialog reusing `components/ui/Dialog`,
essentially `CreateProjectDialog`'s shape pre-filled with the current
values. Cancel is free — the dialog's `name`/`description` state is local
and simply discarded on unmount, no backend call, no store mutation. Save
calls `updateProject`, then the parent's `onUpdated(project)` — which
both updates `activeProject` (workspace shows the new values immediately)
and moves the project to the front of `ProjectsView`'s local `projects`
array (matching the backend's `updated_at DESC` order without a
re-`listProjects()` round trip). On error, the dialog stays open with the
user's typed values intact and shows the error inline — nothing is
silently reverted. On `AppError::NotFound` specifically (the project was
deleted elsewhere), `onNotFound` fires instead: the workspace closes and
the stale entry is dropped from the list, rather than showing a confusing
generic error for a record that's already gone.

**Delete, revisited:** now triggered only from inside the Workspace (not
duplicated in the list — see DECISIONS.md for why). Same confirm-dialog
UX as TASK-005 (`DeleteProjectDialog`, unchanged), but its `onDeleted`
now also clears `activeProject`, returning the user to the Projects list.
`AppError::NotFound` here is treated as an effective success (the record
being gone either way, this is the exact state a working delete leaves
you in) rather than an error.

**Testing:** 44 Rust tests (35 in `repositories`/`services` for the
Project domain plus infra tests — up from 24 before this task; new:
`update_changes_name_and_description`,
`update_does_not_change_id_or_created_at`,
`update_missing_project_returns_false`,
`updating_a_project_moves_it_to_the_top_of_the_list`, and 6
`services::project` update tests covering trimming, validation,
`updated_at` regeneration/`created_at` preservation, and not-found), all
against isolated `tempfile::tempdir()` databases. The full UI flow (empty
→ create → enter workspace → Overview displays → edit → cancel discards →
edit → save persists and reorders the list → return to Projects → delete
from workspace with confirmation → returns to Projects → empty state)
was verified against the Vite dev server with the same mocked-IPC
approach as TASK-005, extended to also mock `update_project`. See
PROJECT_STATE.md for the full record, including a light real-`AppData`
spot-check of the update path specifically (no repeat of TASK-005's
extensive investigation, per this task's own instruction).

## Status

Reflects the state after **TASK-006** (Project Workspace and editing —
still no Process/Capture/Recording/AI functionality). See
[PROJECT_STATE.md](../PROJECT_STATE.md) for the authoritative current
implementation status.
