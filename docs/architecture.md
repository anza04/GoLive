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
holds the one generic piece reused so far, `EmptyState`. `pages/` holds
`ProjectsPage` and `SettingsPage` (both placeholders — no data). `types/`
holds `navigation.ts`. `services/foundation.ts` is still the only Tauri
wrapper. `stores/` and `utils/` remain empty — nothing needs them yet.

## 3. Current Rust/Tauri structure

**CURRENT:**

```
src-tauri/
  migrations/
    0001_initial.sql        infrastructure-only migration (see §17)
  src/
    main.rs                 entry point
    lib.rs                  Tauri builder, .setup() hook, command registration
    errors.rs                AppError (see §10)
    commands/
      foundation.rs           check_foundation_status
      storage.rs               get_local_storage_status
    db/
      mod.rs                   DbService: init, pool access
      pool.rs                  r2d2 pool construction + PRAGMAs
      migrations.rs            migration runner
    repositories/
      storage_status.rs        StorageStatusRepository trait +
                                SqliteStorageStatusRepository
```

`commands/`, `db/`, and `repositories/` were introduced by TASK-004 — the
first modules with enough real content to justify splitting out of
`lib.rs`. There is still no `services/` (no business/domain logic exists
yet — `db::DbService` is infrastructure, not domain logic) and no
`models/` (no domain structs like `Project` exist yet). Both are
introduced when the first real business logic/domain model needs them
(expected TASK-005).

**FUTURE:**

- **`services/`** — Application/business logic. Pure Rust, independently
  testable, no direct knowledge of Tauri's `invoke` plumbing or of
  `repositories/` beyond the trait it depends on.
- **`models/`** — Domain/data structures shared across backend logic
  (`Project`, `Process`, `Capture`, ...).

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

**Concrete examples (both app-level, no feature exists yet):**
- `App.tsx` calls `checkFoundationStatus()` from
  [`src/services/foundation.ts`](../src/services/foundation.ts) →
  `invoke("check_foundation_status")` →
  [`commands::foundation::check_foundation_status`](../src-tauri/src/commands/foundation.rs).
  Infallible, no persistence — no application-service/repository step was
  added around it, since none would do anything.
- `SettingsPage` calls `getLocalStorageStatus()` from
  [`src/services/storage.ts`](../src/services/storage.ts) →
  `invoke("get_local_storage_status")` →
  [`commands::storage::get_local_storage_status`](../src-tauri/src/commands/storage.rs),
  which goes through `SqliteStorageStatusRepository` (§17) — this one
  *is* fallible (`Result<T, AppError>`, §10) and does reach a repository,
  since it's the first command that genuinely needs one.

## 5. Business logic boundary

**Rule:** business rules do not live in React components (or in Tauri
command handlers, once those exist beyond trivial delegation). They live in
Rust application services (see §3), independently of the UI.

Examples of rules that will apply this boundary later (none are
implemented yet):
- "A Capture belongs to zero or one Process."
- "AI-generated process regeneration must not silently overwrite a
  previous process version."

A React component (or a Tauri command) should call a service function and
render/relay the result — it should not itself decide what the rule is.

## 6. Persistence boundary

**CURRENT (infrastructure only, established TASK-004).** SQLite
persistence infrastructure exists — connection pool, migrations, one
repository — but no domain data (Project, Process, Capture, ...) yet.
See §17 for the full detail.

```
Application/domain logic (Rust)     [domain logic itself: FUTURE, TASK-005+]
        ↓
Repository interface (Rust trait)   CURRENT — StorageStatusRepository
        ↓
SQLite implementation               CURRENT — SqliteStorageStatusRepository
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
  migration failures). Each carries a fixed, generic, user-safe message —
  never a raw underlying error.
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

**CURRENT:** `App.tsx` uses local component state (`useState`) for its
connectivity status — the only state in the application. No state
management library is installed.

**Rule:**
- **Local UI state** (modal open/closed, form fields, a single component's
  status) → local component state (`useState`/`useReducer`). Default
  choice.
- **Feature state** (e.g. future active-recording state, process-editor
  state) → owned within that feature, introduced only once the feature
  needs it.
- **Application state** (e.g. future current project, global settings) →
  `src/stores/`, introduced only once at least two features genuinely need
  to share it.

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
capability is granted ahead of the code that needs it.

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
- **Generic reusable UI** (`src/components/ui/`): currently just
  `EmptyState`, reused by both placeholder pages. New components go here
  only once actually reused (see §2's `components/` convention).
- **Pages** (`src/pages/`): `ProjectsPage.tsx` and `SettingsPage.tsx`,
  both empty-state placeholders — no data, no persistence, no business
  logic.
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

## Status

Reflects the state after **TASK-004** (local SQLite persistence
infrastructure — no domain schema, no Project/Capture/Process
functionality). See [PROJECT_STATE.md](../PROJECT_STATE.md) for the
authoritative current implementation status.
