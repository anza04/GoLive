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
  hooks/        small, framework-level React hooks shared across features (§27)
  pages/        top-level routed views composing features
  services/     app-level wrappers around Tauri invoke() calls
  stores/       cross-feature client-side state
  types/        shared TypeScript domain types
  utils/        small pure helpers
  widget/       the floating capture widget's own small React app (§24)
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
- **`hooks/`** (TASK-014) — Small, reusable React hooks with no domain
  ownership, the hook-shaped counterpart to `utils/`. Currently one
  occupant, `useElapsedSeconds` (see §27) — introduced only once a piece
  of UI logic (second-by-second elapsed-time ticking) was genuinely
  needed identically by two independent components, not scaffolded
  ahead of need, same rule as every other shared-code folder here.
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
  ownership.
- **`widget/`** (TASK-011) — the floating capture widget's own root
  component (`Widget.tsx`), entry point (`widget-main.tsx`, mounted by
  `widget.html`, a second Vite/HTML entry alongside `index.html`), and
  stylesheet. Deliberately a top-level sibling of `features/`, not a
  feature inside the main app: it's a genuinely separate small React
  tree running in a second Tauri window with no shared JS memory with
  `App.tsx`'s tree — "a feature of the app" would misdescribe that. It
  still reuses `App.css`'s design tokens/`.button` styles and imports
  directly from `features/projects/services/captures.ts` for the
  capture operations both windows must call identically
  (`createScreenshotCapture`, `createQuickMarker` since TASK-012,
  `startRecordingCapture`/`stopRecordingCapture` since TASK-013) — see
  §24.

**Current contents:** `App.tsx` composes the application shell (see §16),
wrapped in `stores/activeProcess.tsx`'s `ActiveProcessProvider` (TASK-010).
`components/layout/` holds `AppShell`, `Sidebar`, `Header`; `components/ui/`
holds the generic pieces reused across the app so far — `EmptyState`,
`StatusPill`, `Dialog`. `pages/ProjectsPage.tsx` is now a thin wrapper
around the real `features/projects/` feature (see §18);
`pages/SettingsPage.tsx` is still a placeholder. `types/` holds
`navigation.ts`. `services/` holds the app-level Tauri wrappers
(`foundation.ts`, `storage.ts`, `activeProcess.ts`, `widget.ts` — the last
two added TASK-010/011); `features/projects/services/projects.ts` and
siblings hold the feature-specific ones. `utils/` holds `formatDate.ts`
and `errorMessage.ts`. `stores/` holds `activeProcess.tsx` (§11, §23) —
its first occupant, no longer empty. `widget/` (TASK-011, see above) is
the newest top-level folder.

## 3. Current Rust/Tauri structure

**CURRENT:**

```
src-tauri/
  capabilities/
    default.json             main window's capability (core:default)
    widget.json                widget window's capability (see §13, §24)
  migrations/
    0001_initial.sql        infrastructure-only migration (see §17)
    0002_projects.sql        Project domain schema (see §18)
    0003_processes.sql        Process domain schema (see §20)
    0004_captures.sql          Capture domain schema (see §21)
  src/
    main.rs                 entry point
    lib.rs                  Tauri builder, .setup() hook, command registration
    errors.rs                AppError (see §10)
    active_process.rs         ActiveProcessState: cross-window active-
                               Process mirror (see §24)
    hotkey.rs                  global-shortcut registration + handler (§24)
    recording.rs                 RecordingState: in-progress-recording
                                   handle, bridges the two-phase start/
                                   stop commands (see §26)
    commands/
      foundation.rs           check_foundation_status
      storage.rs               get_local_storage_status
      project.rs                create/list/get/update/delete_project (§18)
      process.rs                 create/list/get/update/delete_process (§20)
      capture.rs                  create/list/get/update/delete_capture,
                                    create_screenshot_capture,
                                    get_capture_media (§21, §22)
      active_process.rs           sync_active_process, get_active_process
                                    (§23, §24 — supersedes TASK-010's
                                    commands::tray)
      widget.rs                   hide_widget (§24)
      recording.rs                 start_recording_capture,
                                     stop_recording_capture (§26)
    db/
      mod.rs                   DbService: init, pool access
      pool.rs                  r2d2 pool construction + PRAGMAs
      migrations.rs            migration runner
    media/
      mod.rs                   MediaStorage: captured-media filesystem
                                boundary (see §22, extended §26 for video)
    models/
      project.rs                Project struct (see §18)
      process.rs                  Process struct + ProcessStatus enum (§20)
      capture.rs                   Capture struct + CaptureType enum (§21)
    native/
      mod.rs                   native/platform-specific functionality root
      screenshot.rs            ScreenshotEngine trait +
                                WindowsScreenshotEngine (see §22)
      recording.rs              RecordingEngine/RecordingHandle traits +
                                  WindowsRecordingEngine (see §26)
    repositories/
      storage_status.rs        StorageStatusRepository trait +
                                SqliteStorageStatusRepository
      project.rs                 ProjectRepository trait +
                                  SqliteProjectRepository (see §18)
      process.rs                  ProcessRepository trait +
                                   SqliteProcessRepository (see §20)
      capture.rs                   CaptureRepository trait +
                                    SqliteCaptureRepository (see §21, §22)
    services/
      project.rs                 ProjectService: validation, id/timestamp
                                  generation (see §18)
      process.rs                  ProcessService: validation, id/timestamp
                                   generation, project-existence check (§20)
      capture.rs                   CaptureService: validation, id/timestamp
                                    generation, process-existence check,
                                    screenshot/media orchestration (§21,
                                    §22), recording finalization (§26)
    tray.rs                      system tray icon + menu (see §23, §24)
```

`commands/`, `db/`, and `repositories/` were introduced by TASK-004.
`models/` and `services/` were introduced by TASK-005, once the Project
domain gave them real content — `db::DbService` was always
infrastructure, not domain logic, so it didn't count as a reason to add
`services/` on its own; `ProjectService` is the first actual occupant.
TASK-007 added a sibling `process.rs` file to each of these modules for
the Process domain, following the exact same shape; TASK-008 added a
sibling `capture.rs` file the same way for the Capture domain. TASK-009
added two new top-level modules — `media/` (filesystem storage for
captured media, analogous to `db/`) and `native/` (platform-specific
capture engines, the first real occupant of the "Native Windows
functionality boundary", §9) — rather than folding either into an
existing module, since neither is domain logic (`services/`),
persistence-via-SQLite (`repositories/`), or a Tauri command (`commands/`).
TASK-010 added a third top-level module, `tray.rs` — infrastructure that
owns one native OS resource (the tray icon) for the app's whole lifetime,
the same "own a resource, expose a small typed handle via managed state"
shape `db::DbService`/`media::MediaStorage` already established, not
domain logic or persistence either. TASK-011 added two more,
`active_process.rs` (the cross-window active-Process mirror, same
managed-state shape again) and `hotkey.rs` (the global-shortcut
registration + handler — `native/`'s sibling for a *global*, not
per-display, native capability), plus a second capability file
(`capabilities/widget.json`) and a second `commands/` file
(`active_process.rs`, `widget.rs`) — see §24 for the full picture.
`commands::tray::set_active_process_tray` (TASK-010) no longer exists:
`commands::active_process::sync_active_process` supersedes it, carrying
full Process/Project identity (not just display names) so the floating
widget can act on the active Process, not only show it.

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

**CURRENT.** SQLite persistence infrastructure (TASK-004) now has two
related domain occupants: Project (TASK-005) and Process (TASK-007),
related 1:N with `Process.project_id` a foreign key
(`ON DELETE CASCADE` — see §20). See §17 for the infrastructure, §18 for
Project, §20 for Process.

```
Application/domain logic (Rust)     CURRENT — ProjectService (§18),
        ↓                                     ProcessService (§20)
Repository interface (Rust trait)   CURRENT — StorageStatusRepository,
        ↓                                     ProjectRepository,
        ↓                                     ProcessRepository
SQLite implementation               CURRENT — SqliteStorageStatusRepository,
                                                SqliteProjectRepository,
                                                SqliteProcessRepository
```

The frontend never knows SQLite exists — it only ever calls a Tauri
command. A future cloud architecture should be able to replace the SQLite
repository implementation with a remote API/server repository behind the
same trait, without changing the command layer or the UI.

## 7. File storage boundary

**PARTIALLY CURRENT (TASK-009, extended TASK-013).** Screenshot PNGs
were the first real occupant; Recording MP4s (see §26) are the second —
both live in `media::MediaStorage`, which owns
`<app_data_dir>/captures/`, a sibling of `<app_data_dir>/database/`.
Audio, project files, and exported documents remain **FUTURE**.

**Rule:** native filesystem operations live in the Rust/native layer
(behind `MediaStorage`, analogous to the repository boundary in §6),
never directly in React — see §22 for the concrete instantiation of this
rule.

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

**PARTIALLY CURRENT (TASK-009, TASK-011, TASK-013).** Screenshot capture
(`native::screenshot::ScreenshotEngine`, §22), the global capture
shortcut (`hotkey.rs`, §24), and screen recording
(`native::recording::RecordingEngine`, §26) are real. Microphone capture
remains **FUTURE**.

**Rule:** this functionality is implemented as Rust/Tauri services exposed
through commands. React consumes a clean application API (e.g. "start
recording", "take screenshot") — it never manipulates Windows APIs
directly, as demonstrated by `create_screenshot_capture` (§22): the
frontend asks for a screenshot, the native `xcap`-based engine and
Windows GDI details never surface above `commands::capture`. The global
shortcut (§24) goes one step further: it has no frontend involvement in
triggering a capture *at all* — no window's JS ever touches
`tauri-plugin-global-shortcut` — since a global hotkey has no
"requesting window" a command model assumes.

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
connectivity status. `ProjectsView` owns `activeProject` (TASK-005 called
this `selectedId`; TASK-006 renamed/promoted it to hold the full
`Project`, since the workspace needs the whole record). Nested one level
deeper, `ProcessesView` (rendered by `ProjectWorkspace`'s "Processes" tab,
TASK-007) owns its own `selectedId: string | null` — deliberately the
simpler, TASK-005-style shape rather than a second "active process"
promoted to a full record, since Processes is a list+detail pane, not
another full workspace/back-navigation layer (see §20). Both remain
feature-local. `src/stores/` got its first real occupant in TASK-010 —
see below and §23. No state management library is installed.

**Rule:**
- **Local UI state** (modal open/closed, form fields, a single component's
  status) → local component state (`useState`/`useReducer`). Default
  choice. Example: `CreateProjectDialog`/`EditProjectDialog`'s form fields
  and submitting flag.
- **Feature state** (e.g. process-editor state) → owned within that
  feature, introduced only once the feature needs it. Example:
  `activeProject` in `ProjectsView` — deliberately just
  `useState<Project | null>`.
- **Application state** (genuinely cross-feature) → `src/stores/`,
  introduced only once at least two consumers genuinely need to share it.
  `stores/activeProcess.tsx` (TASK-010) is the first occupant — "which
  Process is active" is read by `ProcessesView`/`ProjectsView` (to set/
  clear it) and, outside React entirely, by the system tray via
  `services/tray.ts` (see §23) — exactly the "future feature needing the
  current process too" scenario this section anticipated back in TASK-007,
  now real. Implemented as a plain React Context
  (`ActiveProcessProvider`/`useActiveProcess`, mounted once in `App.tsx`)
  rather than a library — still "the simplest option that fits," per this
  section's original plan, not a reason to add a dependency.

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
(scoped to the `main` window) grants `core:default` only. TASK-011 added
a second file, [`capabilities/widget.json`](../src-tauri/capabilities/widget.json)
(scoped to the `widget` window only — capabilities are per-window, see
`"windows"` in each file), granting `core:default` plus
`core:event:allow-listen` — the one additional permission the floating
widget genuinely needs, to receive the active-Process/screenshot-result
events Rust broadcasts (see §24). No filesystem, shell, microphone,
screen-capture, or HTTP permission is requested by either window.
`global-shortcut:*` permissions were deliberately **not** added anywhere:
the shortcut is registered and handled entirely in Rust (`hotkey.rs`) —
no window's JS ever calls the plugin's IPC-facing commands, so no
capability grant was needed for it (Rust-side native API calls aren't
gated by the frontend ACL at all; only IPC crossing into the webview is).

**Rule:** each future capability (filesystem access for project storage,
microphone, screen capture, HTTP for the AI provider, etc.) is added only
in the task that implements the corresponding functionality, scoped as
narrowly as Tauri allows for that feature — and, as of TASK-011, scoped
to the specific window that actually needs it, not granted app-wide by
default. No capability is granted ahead of the code that needs it. The
Project commands (TASK-005) needed none of these — they only touch the
already-managed SQLite connection — so capabilities were unchanged then.

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
| `@tauri-apps/api` | `invoke()` and other frontend↔Tauri bindings. TASK-011 is the first to use its `event` sub-module (`listen()`, in `services/activeProcess.ts`/`features/projects/services/captures.ts`) alongside `core` — not a new package, already covered by this one. |
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
| `serde_json` | JSON value handling — now used directly (`hotkey::tests` pins the `CaptureResult` wire shape via `serde_json::to_string`), on top of the original reason it was kept (near-certain need once AI/JSON payloads exist), see TASK-002 decision. |
| `rusqlite` (`bundled`) | Embedded SQLite bindings. `bundled` statically compiles SQLite into the binary — no system SQLite install required (see §17, DECISIONS.md). |
| `r2d2`, `r2d2_sqlite` (`bundled`) | Connection pool for `rusqlite`, so one long-running database operation doesn't block every other one (see §17, "Concurrency"). |
| `thiserror` | Derives `AppError`'s `Display`/`std::error::Error` impl (§10) with minimal boilerplate. |
| `uuid` (`v4`) | Generates `Project` ids (§18). Already an indirect dependency of the toolchain; added as a direct one now that our own code (`ProjectService`) actually calls it. |
| `xcap` (`image` feature) | TASK-009: captures the primary Windows display and encodes it as PNG (`native::screenshot::WindowsScreenshotEngine`, §22). Chosen over hand-rolling a GDI capture directly against the `windows` crate — `xcap` already wraps that (plus macOS/Linux backends we don't use, but don't pay extra for either — they're behind per-platform `[target.'cfg(...)'.dependencies]` and never compiled into the Windows build) behind a small, actively maintained API (`Monitor::all()` / `capture_image()`) that does exactly what this task needs and nothing more; it re-exports the `image` crate it already depends on (`xcap::image`), so no separate `image` dependency was added. `xcap`'s optional Windows-Graphics-Capture backend (`wgc` feature, extra Direct3D/DXGI bindings) was deliberately **not** enabled — the default GDI backend is the simplest reliable option for "capture the primary display," and TASK-009 explicitly scopes to that one mode (see DECISIONS.md). |
| `tauri-plugin-global-shortcut` (target `cfg(any(macos, windows, linux))`, matching the plugin's own documented Cargo.toml line) | TASK-011: registers and handles the global capture keyboard shortcut (`hotkey.rs`, §24). Tauri's own first-party plugin for this — no alternative crate was evaluated, the same way `tray-icon` (TASK-010) needed no alternatives-considered analysis. Only its Rust API is used (`GlobalShortcutExt`, `Builder::with_handler`); the JS-side `@tauri-apps/plugin-global-shortcut` package was deliberately not added, since no window's frontend registers, unregisters, or otherwise touches shortcuts — see DECISIONS.md. |
| `windows-capture` | TASK-013: records the primary Windows display and encodes it to MP4 (`native::recording::WindowsRecordingEngine`, §26). Wraps the Windows Graphics Capture API plus a hardware-accelerated Media Foundation video encoder — no external runtime (no bundled ffmpeg) required. Chosen over extending `xcap` (already a dependency) with its own `video_recorder()`, which delivers only raw, unencoded frames and was still an upstream work-in-progress at the time this was written — pairing it with a *separate* encoding crate would have meant evaluating and adding two new dependencies working together instead of one that already does both; see DECISIONS.md for the full comparison, including why a bundled-ffmpeg approach was rejected. |

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

## 20. Process domain (TASK-007)

**CURRENT.** The second real GoLive domain entity, related 1:N to
Project:

```
Project 1 ─────── N Process
```

Process is the future parent/context for Captures, screen/microphone
recordings, transcripts, and AI-generated analysis (see §7, §8, §9) —
none of those are implemented yet. This task only establishes Process
itself: creation, listing, retrieval, update, deletion, ordering, and a
basic three-state lifecycle status.

**Where Process lives in the Workspace:**

```
Project Workspace
    ├── Overview        implemented (§19)
    ├── Processes        implemented (this section) — list + create +
    │                      select + detail + edit (incl. status) + delete
    ├── Captures         NOT AVAILABLE YET (disabled tab)
    └── Documentation    NOT AVAILABLE YET (disabled tab)
```

**Model** (`models/process.rs`): `Process { id, project_id, name,
description, status, created_at, updated_at }`.
- `id`, `created_at`, `updated_at`: same backend-generated,
  never-frontend-authoritative convention as `Project` (§18).
- `project_id`: set once at creation (after verifying the project
  exists — see below), never changeable through update. A Process
  belongs permanently to its Project; moving a Process between projects
  is not implemented.
- `status`: `ProcessStatus` — a real Rust enum (`Draft`, `InProgress`,
  `Completed`), not a raw string used ad hoc in business logic. It
  implements `rusqlite`'s `ToSql`/`FromSql` directly (so the repository
  binds/reads it as a typed column, not a manually-converted `String`)
  and derives `Serialize`/`Deserialize` with `rename_all = "snake_case"`,
  so the exact same three lowercase strings (`draft`, `in_progress`,
  `completed`) are used in SQLite, in the Rust enum's wire format, and in
  the frontend's `ProcessStatus` TypeScript union — one representation,
  three layers, no ad hoc mapping. No automatic status transitions exist;
  the user changes status explicitly via the edit dialog.

**Schema** (`migrations/0003_processes.sql`, additive — `0001`/`0002`
untouched): `processes` table with
`project_id TEXT NOT NULL REFERENCES projects(id) ON DELETE CASCADE`,
indexed on `project_id` and on `(project_id, updated_at DESC)` (the
actual, only listing query: "processes for project X, most recently
updated first"). **Deleting a Project cascades to delete its Processes at
the database level** — `ProjectRepository::delete` has zero knowledge
that processes exist; the foreign key (combined with `PRAGMA
foreign_keys = ON`, already set on every pooled connection since §17)
owns this behavior entirely. Proven by
`repositories::process::tests::deleting_a_project_cascades_to_its_processes`.

**Repository** (`repositories/process.rs`): `ProcessRepository` trait —
`create`, `list_by_project` (ordered `updated_at DESC`, scoped to one
project), `get`, `update`, `delete` — same shape as `ProjectRepository`
(§18): `update`/`delete` return whether a row existed; `update`'s `SET`
clause never touches `id`, `project_id`, or `created_at`.

**Service** (`services/process.rs`): `ProcessService` holds **two**
repositories — `Box<dyn ProcessRepository>` and
`Box<dyn ProjectRepository>` — because `create` needs to confirm the
parent project exists before writing anything (docs section "Project
ownership"). That check gives a clear `AppError::NotFound` instead of
relying solely on the foreign key to reject an orphaned insert with a
generic database error (the FK still exists as a defense-in-depth
backstop — the explicit check is about error quality, not correctness).
`create` sets `status: Draft` unconditionally; the frontend cannot
specify status when creating. `update` parses the incoming status string
via `ProcessStatus::parse`, returning `AppError::Validation` for anything
else — an arbitrary frontend-supplied string is never accepted.

**Commands** (`commands/process.rs`): `create_process`, `list_processes`,
`get_process`, `update_process`, `delete_process` — thin, delegate to
`ProcessService`. `list_processes` takes an explicit
`ListProcessesInput { project_id }` rather than a bare `project_id:
String` parameter — deliberately avoiding reliance on Tauri's default
camelCase argument-name conversion for a multi-word parameter, a detail
this project's mocked-IPC frontend verification can't independently
confirm (see DECISIONS.md). `update_process` similarly takes an explicit
`UpdateProcessInput { id, name, description, status }` — no
`project_id`/`created_at`/`updated_at` field for a request to occupy.

**Frontend service** (`features/projects/services/processes.ts`): typed
`createProcess`/`listProcesses`/`getProcess`/`updateProcess`/
`deleteProcess`, mapping wire-format `RawProcess` (`snake_case`) to a
`Process` type (`camelCase`) — `status` needs no translation, since the
backend already serializes it as one of the three strings the frontend's
`ProcessStatus` union expects directly.

**Frontend UI** (`features/projects/components/`):
- `ProjectWorkspace` now tracks real `activeTab` state (`useState`,
  previously a fixed constant since only one tab was ever enabled) — the
  `WORKSPACE_TABS` array gained an `available` flag per tab; clicking a
  `disabled` tab is a no-op (native `<button disabled>` behavior).
  `ProjectsView` mounts `ProjectWorkspace` with `key={project.id}`, so
  switching to a different project resets `activeTab` back to
  "overview".
- `ProcessesView` (rendered by the "Processes" tab): list + create entry
  point + loading/error/empty states, structurally identical to
  TASK-005's original `ProjectsView` shape (list + detail pane, not a
  nested workspace) — see "Process selection state" below for why.
- `ProcessList`, `ProcessDetail` (name, description,
  `ProcessStatusBadge`, dates, Edit/Delete actions, and two reserved
  informational cards — "Captures" / "AI analysis" — reusing the generic
  `.reserved-sections` CSS from §19).
- `ProcessStatusBadge` — small reusable label (`Draft` / `In progress` /
  `Completed`); internal enum formatting is never shown directly.
- `CreateProcessDialog` (no status field — Draft is implicit),
  `EditProcessDialog` (adds a status `<select>` to the
  name/description fields `EditProjectDialog` already established),
  `DeleteProcessDialog` — all reuse `components/ui/Dialog`.
- `ProjectOverview`'s reserved-sections list dropped "Processes" — it's
  no longer a placeholder, it's real.

**Process selection state:** kept as a plain `selectedId: string | null`
in `ProcessesView`, not promoted to holding the full `Process` object the
way `ProjectsView`'s `activeProject` was in TASK-006. Processes doesn't
get its own nested workspace — selecting one shows a detail pane
alongside the list (this task's "keep the architecture simple"
instruction) — so there's no separate component that needs the full
record passed down; `.find()` over the already-loaded list is enough.

**Errors:** reuses `AppError::Validation` (empty/over-length
name/description, invalid status string) and `AppError::NotFound`
(missing project on create, missing process on get/update/delete) — no
new `AppError` variants were needed. `EditProcessDialog` and
`DeleteProcessDialog` handle `NotFound` gracefully via
`isNotFoundError()`, same pattern as the Project dialogs (§19).

**Testing:** 63 Rust tests (was 44) — 19 new: 11 repository tests
(create/list-by-project/list-ordering/update incl.
id-project_id-created_at immutability/delete/not-found variants/cascade
delete/reopen-persistence) and 15 service tests (create incl. Draft
default and trimming, all four validation-rejection cases, missing
project, update incl. status change and invalid-status rejection,
updated_at regeneration + created_at/project_id preservation, delete),
plus 2 new `db` tests confirming the `processes` table, its indexes, and
its foreign key exist after migrating a fresh database — all against
isolated `tempfile::tempdir()` databases (or, for the cascade test, two
repositories sharing one such database). Full UI flow (Processes tab
enabled, empty state, create, list, select, detail incl. reserved
sections, edit incl. status change and cancel-discards, save reorders the
list to the top, delete with confirmation, empty state returns, project
delete cascades to its processes, Captures/Documentation stay disabled)
verified against the Vite dev server with the same mocked-IPC approach as
TASK-005/006, extended to cover all five Process commands and project-
delete cascade. See PROJECT_STATE.md for the full record, including a
light real-`AppData` spot-check of the Process create/update/list path.

## 21. Capture domain (TASK-008)

**CURRENT.** The third real GoLive domain entity, related 1:N to Process,
completing the hierarchy:

```
Project 1 ─────── N Process 1 ─────── N Capture
```

A Capture is a single piece of evidence/content collected while
documenting a Process — a screenshot, a recording, or a free-form note.
**Metadata only**: no actual screenshot/recording/media file is captured,
stored, or attached by this task — that's later work (screen capture,
microphone recording, global hotkeys, floating widget, transcription, AI
are all explicitly out of scope here). `CaptureType::Screenshot`/
`Recording` describe the *future* kind of media a Capture will eventually
hold, not media that exists today.

**Where Capture lives — nested inside Process, not a new Workspace tab:**

```
Project Workspace
    ├── Overview        implemented (§19)
    ├── Processes        implemented (§20)
    │     └── ProcessDetail
    │           └── Captures section   implemented (this section) — list +
    │                                    create + select + detail + edit +
    │                                    delete, nested inside the already-
    │                                    selected process, not a new
    │                                    Workspace-level tab
    └── Documentation    NOT AVAILABLE YET (disabled tab)
```

The Project Workspace's own "Captures" tab (§19) stays genuinely
`disabled` — Captures belong to a Process, not directly to a Project, so
there is no project-wide capture list yet. The task explicitly scoped
this: "Do NOT create a project-wide capture list yet."

**Model** (`models/capture.rs`): `Capture { id, process_id, capture_type,
title, description, created_at, updated_at }`, serialized to the wire as
`{ id, process_id, type, title, description, created_at, updated_at }`.
- `id`, `created_at`, `updated_at`: same backend-generated,
  never-frontend-authoritative convention as `Project`/`Process` (§18,
  §20).
- `process_id`: set once at creation (after verifying the process exists
  — see below), never changeable through update.
- `capture_type`: `CaptureType` — a real Rust enum (`Screenshot`,
  `Recording`, `Note`), same pattern as `ProcessStatus` (§20):
  hand-written `rusqlite::{ToSql, FromSql}` impls and
  `Serialize`/`Deserialize` with `rename_all = "snake_case"`, so
  `screenshot`/`recording`/`note` is the one representation used in
  SQLite, the Rust wire format, and the frontend's `CaptureType`
  TypeScript union. The Rust field is named `capture_type` (`type` is a
  reserved word) with `#[serde(rename = "type")]`, so the model still
  serializes with the documented key `"type"` and matches the frontend's
  `Capture.type` — see DECISIONS.md for why the command *input* structs
  use the literal field name `capture_type` instead (no rename) rather
  than the same `type`-on-the-wire shape.

**Schema** (`migrations/0004_captures.sql`, additive — `0001`–`0003`
untouched): `captures` table with
`process_id TEXT NOT NULL REFERENCES processes(id) ON DELETE CASCADE`,
indexed on `process_id` and on `(process_id, updated_at DESC)` (the
actual, only listing query: "captures for process X, most recently
updated first"). **Deleting a Process cascades to delete its Captures at
the database level**, and because `processes.project_id` already cascades
from `projects` (§20), deleting a Project transitively cascades through
two chained `ON DELETE CASCADE` relationships — Project → Process →
Capture — with no application-side cascade loop at either level. Proven
by `repositories::capture::tests::deleting_a_process_cascades_to_its_captures`
and `deleting_a_project_cascades_through_processes_to_captures`.

**Repository** (`repositories/capture.rs`): `CaptureRepository` trait —
`create`, `list_by_process` (scoped to one process, `updated_at DESC`),
`get`, `update`, `delete` — same shape as `ProcessRepository`/
`ProjectRepository`: `update`/`delete` return whether a row existed;
`update`'s `SET` clause never touches `id`, `process_id`, or
`created_at`.

**Service** (`services/capture.rs`): `CaptureService` holds **two**
repositories — `Box<dyn CaptureRepository>` and
`Box<dyn ProcessRepository>` — for the same reason `ProcessService` holds
`ProjectRepository` (§20): `create` confirms the parent process exists
(clean `AppError::NotFound` instead of relying solely on the foreign key)
before writing. `title` (required, ≤200 chars) and `description`
(optional, ≤5000 chars) use the same trim/validate rules as
`name`/`description` elsewhere; `capture_type` is parsed via
`CaptureType::parse`, rejecting anything else with
`AppError::Validation` — an arbitrary frontend-supplied type string is
never accepted.

**Commands** (`commands/capture.rs`): `create_capture`, `list_captures`,
`get_capture`, `update_capture`, `delete_capture` — thin, delegate to
`CaptureService`. `list_captures` takes an explicit
`ListCapturesInput { process_id }` (same rationale as
`ListProcessesInput`, §20) and `update_capture` an explicit
`UpdateCaptureInput { id, capture_type, title, description }` — no
`process_id`/`created_at`/`updated_at` field for a request to occupy.

**Frontend service** (`features/projects/services/captures.ts`): typed
`createCapture`/`listCaptures`/`getCapture`/`updateCapture`/
`deleteCapture`, mapping wire-format `RawCapture` (`snake_case`) to a
`Capture` type (`camelCase`) — `type` needs no translation, since the
backend already serializes it as one of the three strings the frontend's
`CaptureType` union expects directly.

**Frontend UI** (`features/projects/components/`):
- `ProcessDetail`'s old "Captures" *reserved* card (§20) is gone —
  replaced by the real `CapturesSection` component, rendered directly
  inside `ProcessDetail` below the process metadata. The "AI analysis"
  reserved card stays, unaffected.
- `CapturesSection` (list + detail pane, its own loading/error/empty
  states, "+ New capture" entry point) — structurally identical to
  `ProcessesView` (§20), just one level deeper and scoped to the current
  process's `processId` instead of a project's `id`. Deliberately not a
  second nested Workspace: no back button, no separate route, per this
  task's "keep the architecture simple" instruction.
- `CaptureList` (title, `CaptureTypeBadge`, updated date), `CaptureDetail`
  (title, description, type, dates, Edit/Delete), `CaptureTypeBadge`
  (`Screenshot`/`Recording`/`Note` — internal enum values never shown
  directly), `CreateCaptureDialog` (Type defaults to Screenshot),
  `EditCaptureDialog` (pre-filled), `DeleteCaptureDialog` — all reuse the
  shared `components/ui/Dialog`, following `CreateProcessDialog`/
  `EditProcessDialog`/`DeleteProcessDialog`'s exact shape.

**Capture selection state:** a plain `selectedId: string | null` inside
`CapturesSection`, same as `ProcessesView`'s `selectedId` (§20) and for
the same reason — a list+detail pane, not a promoted "active capture"
record.

**Errors:** reuses `AppError::Validation` (empty/over-length
title/description, invalid capture type) and `AppError::NotFound`
(missing process on create, missing capture on get/update/delete) — no
new `AppError` variants were needed. `EditCaptureDialog` and
`DeleteCaptureDialog` handle `NotFound` gracefully via
`isNotFoundError()`, same pattern as the Project/Process dialogs.

**Testing:** 95 Rust tests (was 63) — 32 new: 12 repository tests
(create/list-by-process/scoping/ordering/get/update incl.
id-process_id-created_at immutability/delete/not-found variants/reopen-
persistence/cascade-delete-from-process/cascade-delete-through-process-
from-project) and 16 service tests (create incl. trimming, all four
validation-rejection cases, all three valid capture types, invalid
capture type, missing process, update incl. type change and invalid-type
rejection, updated_at regeneration + created_at/id/process_id
preservation, missing-capture, delete), plus 3 new `db` tests confirming
the `captures` table, its indexes, and its foreign key exist after
migration, and one confirming the schema reaches `user_version = 4` — all
against isolated `tempfile::tempdir()` databases. Full UI flow (Processes
tab → create process → select → Captures section empty state → create
capture (Type defaults to Screenshot) → appears, auto-selected → detail
shows title/description/type/dates → Edit: values pre-filled → Cancel
discards (verified against both UI and mock backend state) → Edit again,
change title/type/description → Save persists, badge/dates update
immediately and the capture stays first → create a second capture,
confirm `updated_at DESC` ordering → Delete requires confirmation →
cancel leaves it → delete again removes it and clears selection → delete
the last capture → empty state returns → invalid type/empty title/
over-length title/over-length description all rejected with
`AppError::Validation` → create against a nonexistent process rejected
with `AppError::NotFound` → deleting a Process cascades to its Captures →
deleting a Project cascades through its Processes to their Captures →
Project Workspace's "Captures" tab confirmed still `disabled` via
`element.disabled === true` → Settings still works) was verified against
the Vite dev server with the same mocked-`window.__TAURI_INTERNALS__.invoke`
approach as TASK-005/006/007, extended to cover all five Capture commands.
The native `golive.exe` was built and launched separately and confirmed
running via `tasklist` (killed after verification) against the real,
schema-upgraded (`user_version = 4`) database. Unlike TASK-005/006/007,
no separate multi-process real-`AppData` spot-check was performed for
this task — the automated reopen-persistence and cascade tests already
exercise the identical `DbService`/repository code path against a real
(temp-directory) SQLite file across separate instances, and the compiled
binary was confirmed to start and run cleanly against the real, migrated
per-user database. See PROJECT_STATE.md for the full record.

## 22. Screenshot capture and media storage (TASK-009)

**CURRENT.** The first task to introduce real media: a Screenshot
Capture now has an actual PNG behind it, not just metadata. Note and
Recording Captures remain exactly what TASK-008 left them — metadata
only.

```
Screenshot creation:

Captures UI (CreateCaptureDialog, Type = Screenshot)
    ↓
frontend capture service   features/projects/services/captures.ts
    ↓ createScreenshotCapture()
Tauri invoke("create_screenshot_capture")
    ↓
commands::capture::create_screenshot_capture   thin — delegates
    ↓
CaptureService::create_screenshot   validation, process-existence check,
    ↓                                orchestration (see below)
    ├──→ native::screenshot::ScreenshotEngine   captures the primary
    │        (WindowsScreenshotEngine, xcap)     display, encodes PNG
    ├──→ media::MediaStorage::save_capture       writes the PNG to disk
    └──→ CaptureRepository::create                writes the metadata row
```

**Two new modules, each a small, single-purpose boundary** (see §7, §9):

- **`native::screenshot`** — the native Windows functionality boundary's
  first real occupant. `ScreenshotEngine` is a trait (one method,
  `capture_primary_display() -> Result<Vec<u8>, AppError>`), exactly the
  same "inject a trait object" shape already used for repositories — so
  `CaptureService` (and everything above it) never touches `xcap`, GDI,
  or any Windows-specific detail. `WindowsScreenshotEngine` is the only
  implementation: `xcap::Monitor::all()` → find the primary monitor →
  `capture_image()` → encode to PNG in memory (`image::RgbaImage::
  write_to`, via `xcap`'s own re-export of the `image` crate). Nothing is
  written to disk here — that's `MediaStorage`'s job. TASK-009
  deliberately supports only "capture the primary/current display"; the
  trait is the seam a later task would use to add monitor selection, area
  selection, or a recording engine, without `CaptureService` or the
  frontend changing.
- **`media::MediaStorage`** — the file storage boundary's first real
  occupant (§7). Owns `<app_data_dir>/captures/`, a sibling of
  `<app_data_dir>/database/` under the *same* Tauri-resolved
  application-data directory `db::DbService` already uses — no second
  app-data mechanism was invented. `save_capture`/`read_capture`/
  `delete_capture`/`exists`/`reconcile`, all keyed by Capture id alone:
  `path_for(id)` parses `id` as a UUID before touching the filesystem at
  all and rejects anything else with `AppError::Validation` — the *only*
  path-safety check needed, since a valid UUID string can contain
  nothing but hex digits and hyphens, making path traversal (`../`, an
  absolute path, an arbitrary filename) structurally impossible
  regardless of what a caller passes in. Deliberately has no dependency
  on Tauri types (same rationale as `db::DbService`) — fully
  unit-testable with a temp directory.

**Capture metadata vs. media — deliberately still one domain, two
storage backends**, per the task's own explicit instruction not to
invent a second "Screenshot" domain: SQLite continues to own `Capture`
metadata (unchanged schema — **no migration was needed**, TASK-009's
Capture schema is exactly TASK-008's); the filesystem owns the PNG
bytes. The only link between them is `Capture.id`: `captures/<id>.png`.
There is no path column, no foreign key, no join — the relationship is
structural, not stored.

**`CreateScreenshotInput { process_id, title, description }`**
(`commands::capture`) — deliberately has **no** `capture_type` field at
all, unlike the generic `CreateCaptureInput`. A screenshot operation
always produces `CaptureType::Screenshot`; there is structurally no way
for a request to ask for a screenshot capture typed `recording`/`note`.
No filesystem path of any kind is ever accepted from the frontend — the
backend determines storage location, filename, id, and timestamps
entirely on its own, the same "backend is the only source of these
values" rule §18 established for `id`/`created_at`/`updated_at` extended
to cover storage location too.

**Screenshot creation is transactional at the application level**
(`CaptureService::create_screenshot`) — not a real database transaction
(the media is filesystem data SQLite doesn't participate in), but
ordered, deliberate service-level orchestration:
1. Validate `title`/`description`, confirm the parent Process exists.
2. Capture the display (`ScreenshotEngine`). If this fails, nothing has
   been created yet — nothing to roll back.
3. Save the PNG (`MediaStorage::save_capture`), *before* the metadata
   row — so a metadata-creation failure has something concrete to clean
   up next, rather than a Capture row implying media that doesn't exist.
4. Create the metadata row (`CaptureRepository::create`). If this fails,
   the just-written PNG is deleted (best-effort, logged if that also
   fails) so no orphan media is left behind.

Proven by `services::capture::tests::
create_screenshot_leaves_no_orphan_capture_when_engine_fails` (step 2
failing) and `create_screenshot_cleans_up_the_png_when_metadata_insert_fails`
(step 4 failing, using a `CaptureRepository` test double whose `create`
always errors).

**Safe media access** (`get_capture_media` command) — the frontend never
receives or sends a filesystem path. It supplies only a Capture id;
`CaptureService::get_screenshot_media` derives the file entirely through
`MediaStorage`. Returns `AppError::NotFound` uniformly whether the
Capture doesn't exist at all or exists but has no media (Note/Recording,
or a Screenshot Capture since edited to another type) — the frontend
treats both the same way (no preview to show). The command returns
`tauri::ipc::Response` (raw bytes) rather than a JSON byte array — Tauri
2's documented mechanism for transferring binary data efficiently,
avoiding a ~3-4x JSON-array size/parse cost for a multi-hundred-KB PNG;
`invoke()` on the frontend receives it as an `ArrayBuffer` transparently.
`features/projects/services/captures.ts`'s `getCaptureMediaUrl` wraps
that into a `blob:` object URL an `<img>` can use directly, and documents
that the caller must `URL.revokeObjectURL` it once done —
`CaptureDetail.tsx`'s effect cleanup does exactly that whenever the
selected capture (or its type) changes or the component unmounts.

**Delete behavior** — `CaptureService::delete` deletes the metadata row
first; only if that reports "a row was actually deleted" does it then
best-effort delete the media file. A missing media file (Note/Recording,
which never had one) is a graceful no-op inside `MediaStorage::
delete_capture`, not an error, so `delete` needs no branch on
`capture_type`. A genuine media-cleanup I/O failure is logged, never
surfaced to the frontend — the metadata (the source of truth for whether
a Capture exists) is already gone, which is what "deleted" means to the
user; raw filesystem errors are never shown, consistent with §10.

**Cascade media cleanup — a documented limitation, not a workaround.**
Project → Process → Capture metadata deletion is still owned entirely by
SQLite's chained `ON DELETE CASCADE` foreign keys (§20, §21) — TASK-009
does **not** add a `CaptureRepository`/media dependency to
`ProcessService` or `ProjectService` to delete files synchronously during
a cascade, which the task explicitly warned against as an inefficient,
boundary-violating "for every capture: delete file" loop bolted onto an
unrelated domain's delete path. The database cascade removes the *rows*;
it was never going to know about files on disk. Instead, orphaned PNGs
left behind by a Process/Project cascade are swept by a **startup
reconciliation pass**: `lib.rs`'s `.setup()` builds a `CaptureService`
and calls `reconcile_media()` once, which compares every `.png` file
under `captures/` against every Capture id `CaptureRepository::
list_all_ids()` currently returns, and deletes whatever isn't referenced.
This is the "clean media-storage cleanup boundary" the task asked for —
generic (works for any orphaned media file, not just screenshots — ready
for recordings later), and O(files + rows) once per launch, not a loop
inside a delete call. A reconciliation failure is logged and never blocks
startup. **Limitation, stated plainly:** a PNG orphaned by a cascade
delete is not removed *immediately* — it is removed the next time GoLive
starts. Proven by `services::capture::tests::
reconcile_media_removes_files_orphaned_by_a_cascade_delete`, which
deletes a Capture row directly through the repository (bypassing
`CaptureService::delete`, which already cleans up synchronously) to
simulate exactly what a cascade leaves behind, then calls
`reconcile_media` and asserts the orphan is gone and the still-referenced
file survives. Direct Capture deletion (the common case — the user
pressing Delete on a Capture) remains synchronous, as required.

**Editing a screenshot Capture** — `CaptureService::update` is completely
unchanged from TASK-008: it edits `title`/`description`/`capture_type`
metadata only, never touches media, and never recaptures. Because
`captures/<id>.png` is keyed by `Capture.id` alone (not by
`capture_type`), changing a Screenshot Capture's type to Note/Recording
(or back) doesn't move, delete, or replace anything — the file simply
stops (or starts) being reachable through `get_screenshot_media`/shown by
`CaptureDetail`'s preview, purely because the frontend only requests
media for `type === "screenshot"`. This was a deliberate choice — see
DECISIONS.md — over silently deleting media on a type-away edit, which
the task explicitly warned against. Proven by `services::capture::tests::
update_changing_type_away_from_screenshot_does_not_delete_media`.

**UI** (`features/projects/components/`):
- `CreateCaptureDialog` — same three fields (Title/Description/Type) as
  TASK-008, Type still defaults to Screenshot. When Type is Screenshot,
  a hint ("This will capture your current screen.") appears and the
  primary button reads "Capture screenshot" / "Capturing…" instead of
  "Create capture" / "Creating…", and submission calls
  `createScreenshotCapture` instead of the generic `createCapture` — for
  Note/Recording, behavior is byte-for-byte what TASK-008 shipped.
- `CaptureDetail` — for `type === "screenshot"`, fetches and shows the
  PNG (loading state while the IPC call is in flight, inline error state
  if it fails, an `<img>` once ready) between the description and the
  type/date metadata row, as the task's own recommended layout specifies.
  The image is bounded (`max-height: 360px`, `object-fit: contain`)
  inside a rounded, bordered container — scales down, preserves aspect
  ratio, never forces the surrounding layout to overflow. Note/Recording
  captures render nothing new here — the same metadata-only detail
  TASK-008 shipped.
- `EditCaptureDialog`, `DeleteCaptureDialog`, `CaptureList`,
  `CaptureTypeBadge`, `CapturesSection` — **unchanged** from TASK-008.
- The Project Workspace's "Captures" tab (§19) remains genuinely
  `disabled` — untouched by this task, per its explicit instruction.

**Errors:** one new `AppError` variant, `Capture(String)` — same shape as
`Validation` (author-written, safe, shown as-is), for native
capture-engine failures (no display available, PNG encoding failed) that
don't fit `Storage`/`Database`/`Validation`/`NotFound`. Filesystem
failures *storing* media (as opposed to *capturing* it) reuse `Storage`
via the existing `From<std::io::Error>` impl — `MediaStorage`'s
directory-creation/read/write/delete calls are conceptually the same
category of failure as the database file being unavailable, so no new
variant was needed there. See DECISIONS.md for the full reasoning,
including why `xcap::XCapError` converts to `AppError::Capture` and why
`get_screenshot_media`/`read_capture` collapse "wrong type" and
"nonexistent" into the same `NotFound`.

**Dependency:** `xcap` (`image` feature) — see §15 for the full
why/alternatives-considered entry.

**Testing:** 117 Rust tests (was 95) — 22 new: 11 `media::tests`
(directory creation + idempotence, save/read round-trip, exists,
filename derived from id, missing-file read/delete handled gracefully,
path-traversal rejection for several malicious ids, reconcile removing
only orphaned `.png` files and leaving non-PNG files alone, reconcile on
a missing directory), 1 new repository test (`list_all_ids`), and 10 new
service tests (metadata-only `create` never touches media,
`create_screenshot` end-to-end incl. type/title, missing-process and
empty-title rejection, the two transactional-cleanup tests above, type
change preserving media, delete removing both metadata and media,
deleting a metadata-only capture with no media succeeding gracefully, and
the cascade-reconciliation test) — all against isolated
`tempfile::tempdir()` state, using a `FakeScreenshotEngine`/
`FailingScreenshotEngine` test double for `ScreenshotEngine` and a
`FailingCreateCaptureRepository` decorator for the metadata-failure case.

**Screenshot capture testing limitation, stated explicitly:** real native
screen capture cannot be exercised deterministically inside the automated
suite the way SQLite-backed logic can — there is no guarantee any given
CI/agent process has an interactive desktop session for `xcap`/GDI to
capture. `native::screenshot::tests::capture_primary_display_smoke_test`
exists specifically to exercise `WindowsScreenshotEngine` for real, but
is `#[ignore]`d so `cargo test` never depends on one being available; run
manually with `cargo test -- --ignored capture_primary_display_smoke_test`
on a machine with a real desktop session. See PROJECT_STATE.md for the
manual verification actually performed for this task (the smoke test
did pass in this session's environment, and a further one-off spot-check
confirmed the captured PNG contains real, correct desktop content — not
a placeholder — plus a full real-`%APPDATA%` create → simulated-restart
→ delete cycle through the actual production `CaptureService`/
`MediaStorage` code, proving the whole pipeline, not just `xcap` in
isolation).

## 23. Background persistence, system tray, and the active-process store (TASK-010)

**CURRENT.** The first milestone-M1 step (see roadmap.md): lets GoLive
keep running with its main window hidden, and introduces the app's first
genuinely cross-feature client state.

```
Closing the main window:

React (nothing — this is Rust-only)
    ↑
tauri::Builder::on_window_event   WindowEvent::CloseRequested
    ↓                              → api.prevent_close() + window.hide()
window stays hidden, process keeps running

Reopening / quitting:

Tray icon menu ("Open GoLive" / "Quit")
    ↓
tray::build's on_menu_event       "open" → window.show()+set_focus()
                                    "quit" → app.exit(0)   (the only real exit,
                                              besides a window-manager force-close)

Reflecting the active Process in the tray:

ProcessesView (select/create/rename) or ProjectsView (delete)
    ↓
stores/activeProcess.tsx           ActiveProcessProvider — React Context
    ↓ setActiveProcess()/clearActiveProcess()
services/tray.ts                   syncActiveProcessTray()
    ↓ invoke("set_active_process_tray")
commands::tray::set_active_process_tray   thin — delegates
    ↓
tray::TrayHandles::set_active_process      updates tooltip + menu label live
```

**Tray icon** (`tray.rs`, Cargo feature `tray-icon` on the `tauri`
dependency — no third-party crate, this is Tauri's own built-in
capability): built once in `.setup()` via `TrayIconBuilder`, with a
three-item menu — a disabled, purely informational "Active: …"/"No
active process" label, "Open GoLive", "Quit". The built `TrayIcon` and
the active-label `MenuItem` handle are kept in Tauri-managed state
(`TrayHandles`) so later calls (from the command below) can update them
live via `set_tooltip`/`set_text` — no menu rebuild needed. This is the
same "own one native resource for the app's lifetime, expose it through
managed state" shape `db::DbService`/`media::MediaStorage` already
established (see §17, §22), now for a UI resource instead of a
filesystem/database one.

**Close-to-tray** (`lib.rs`, `Builder::on_window_event`): intercepts
`WindowEvent::CloseRequested`, calls `api.prevent_close()`, and hides the
window instead. Applies to every window the app creates; today there is
only `"main"` (now given an explicit `"label": "main"` in
`tauri.conf.json`, rather than relying on Tauri's implicit default), so
this is effectively main-window-only. **Revisit before TASK-011** adds a
second window (the floating capture widget) — it should very likely not
hide-on-close the same way a "quit the app" click on the main window
should. The tray's "Quit" menu item (`app.exit(0)`) is the only real exit
besides a normal window-manager force-close (`taskkill`, Task Manager,
etc.) — proven manually: sending a graceful close request to the running
process left it running (same PID) rather than exiting; see
PROJECT_STATE.md.

**The active-process store** (`src/stores/activeProcess.tsx`) — the
first real occupant of `src/stores/`, and the concrete case §11 has
anticipated since TASK-007 ("if a future feature — e.g. a floating
widget — needs to know the current project too, this is a same-shape
swap into a `stores/` slice, not a rewrite"). Implemented as a plain
React Context (`ActiveProcessProvider`, mounted once around everything in
`App.tsx`; `useActiveProcess()` hook) — not a state-management library,
per §11's own "simplest option that fits" rule; nothing here needed more
than Context provides. Purely client-side: no backend model, nothing
persisted, cleared back to `null` on every fresh launch. The one thing
outside React that reads it is the system tray, kept in sync as a side
effect of `setActiveProcess`/`clearActiveProcess` (via
`services/tray.ts`) so every call site that changes the active process
automatically keeps the tray correct without remembering to do so itself.

**What sets/clears it** (`ProcessesView.tsx`/`ProjectsView.tsx`):
- Selecting a Process in the list, or a newly created Process being
  auto-selected, marks it active (implicit-on-select was chosen over a
  separate explicit "Set active" action — simplest, and matches "the
  process you're currently looking at is the one you're working on").
- Renaming the *currently active* Process re-marks it active with the
  new name, so the tray label can't go stale after an edit.
- Deleting the active Process, or deleting its parent Project (which
  cascades to the Process — see §20), clears it.
- Everything else (switching which Project's workspace is open,
  selecting a different, non-active Process elsewhere, editing a
  non-active Process) leaves it untouched — the active process is a
  standing "what am I working on" marker, not tied to what's currently
  on screen.

**Errors:** none new. `set_active_process_tray` is intentionally
infallible from the frontend's perspective — updating a tray label is a
cosmetic side effect nothing in the UI needs to react to failing; a
native tray-API failure is logged server-side and otherwise ignored (see
`TrayHandles::set_active_process`).

**Testing:** 119 Rust tests (was 117) — 2 new (`tray::tests`), covering
the one piece of pure logic in `tray.rs` (tooltip/menu-label text
formatting for both the "active" and "no active process" cases) directly,
without a real tray icon — native tray/menu handles can't be constructed
outside a running Tauri app, the same category of limitation §22 already
documents for real screen capture. UI-flow verification (select → tray
call recorded with the right `{ process_name, project_name }` shape →
select a different Process → re-synced → rename the active Process →
re-synced with the new name → delete the active Process → tray call is
`null` → re-select the other Process → delete its *Project* → tray call
is `null` again → Settings and existing Project/Process/Capture flows
unaffected) was verified against the Vite dev server with the same
mocked-`window.__TAURI_INTERNALS__.invoke` approach as every prior task,
extended with a `set_active_process_tray` handler recording its calls.
Close-to-tray was verified against the compiled `golive.exe` directly (a
UI-level pass, not mockable): launched, confirmed running via `tasklist`,
sent a graceful window-close request, and confirmed the *same process*
was still running several seconds later — proof the close was
intercepted and the window hidden rather than the app exiting — then
force-closed for cleanup. Actually clicking the tray icon's "Open
GoLive"/"Quit" menu items could not be verified, the same native-UI
limitation §22 and PROJECT_STATE.md already record (no desktop
UI-automation tool is available in this environment for the native
window/tray, only for a real web browser tab).

## 24. Global hotkey and floating capture widget (TASK-011)

The first step to actually deliver on the product's core premise —
capturing evidence "while users are performing their actual work," per
README.md — rather than requiring the user to switch into GoLive first.
Screenshot only at the time this was built; scope explicitly excluded
recording, audio, and quick markers (quick markers followed immediately
as §25/TASK-012; recording and audio remain later roadmap.md steps).

```
Two ways to trigger a screenshot into the active Process:

(a) Global hotkey (Ctrl+Alt+Shift+S, hardcoded — no customization UI yet)
    ↓ OS-level, no window involved
tauri_plugin_global_shortcut's handler (registered in .setup(), lib.rs)
    ↓
hotkey::handle_capture_shortcut(app)
    ↓ reads active_process::ActiveProcessState directly
    ↓ builds a CaptureService exactly like commands::capture does
CaptureService::create_screenshot()        (unchanged since TASK-009)
    ↓
app.emit("screenshot-captured", result)     Rust-side emit, no ACL involved
    ↓
floating widget listens, shows a brief acknowledgment

(b) Floating widget's own "Capture screenshot" button
    ↓
Widget.tsx (its own window/React tree)
    ↓
createScreenshotCapture()                  features/projects/services/captures.ts
    ↓ invoke("create_screenshot_capture")   — the exact same frontend call
    ↓                                         CreateCaptureDialog (TASK-009)
    ↓                                         makes from the main window
commands::capture::create_screenshot_capture   → CaptureService::create_screenshot()
```

**Two Tauri windows, not one** — `tauri.conf.json` now declares a second
window, `"widget"` (`url: "widget.html"`, a separate Vite/HTML entry —
see §2 — small, `alwaysOnTop`, `decorations: false`, `skipTaskbar: true`,
visible by default). Tauri windows are separate webview processes with
**no shared JS memory**: the widget cannot read the main window's
`stores/activeProcess.tsx` React Context directly, which is the reason
the next piece exists.

**Cross-window active-process sync** (`active_process.rs` +
`commands::active_process`): a small Rust-managed
`ActiveProcessState(Mutex<Option<ActiveProcessInfo>>)` is the mechanism
that bridges the two windows. `sync_active_process` (called by the main
window's `stores/activeProcess.tsx`, superseding TASK-010's narrower
`set_active_process_tray` — now carrying full `process_id`/`project_id`,
not just display names, since the widget needs to *act* on the active
Process, not only show it) updates this state, updates the tray label
(unchanged from TASK-010), and — the new part — broadcasts an
`active-process-changed` event to every window via `AppHandle::emit`
(Rust-side; not gated by the frontend ACL, since only crossing *into* the
webview via IPC is). `get_active_process` lets a window that wasn't open
for the last change (e.g. the widget, just reopened) pull the current
value on mount instead of waiting for the next push. Never persisted —
resets to `None` every launch, same as the store it mirrors.

**The widget window** (`src/widget/`, see §2): its own small React tree,
reusing `App.css`'s design tokens/`.button` styles but with its own
minimal layout (`widget.css`). On mount: fetches the active Process
(`getActiveProcess`), then subscribes to `onActiveProcessChanged` and
`onScreenshotCaptured` (both thin wrappers around
`@tauri-apps/api/event`'s `listen`, in `services/activeProcess.ts` and
`features/projects/services/captures.ts` respectively — components still
never call a raw Tauri API directly, the same rule §4 established for
`invoke`, now extended to `listen`). Shows the active Process's name (or
a "no active process" empty state, button disabled), a "Capture
screenshot" button that calls `createScreenshotCapture` with a fixed
title (`"Screenshot"` — no dialog, matching the hotkey's own no-dialog
behavior so both produce the same kind of Capture), a loading state
while capturing, and inline success/error feedback. Its own "×" button
calls a dedicated `hide_widget` command rather than the raw
`@tauri-apps/api/window` API — sidesteps needing
`core:window:allow-hide` in its capability entirely, since app-defined
commands aren't ACL-gated (see §13). Showing it again is tray-only (the
"Toggle Widget" menu item, `tray.rs`) — the widget never shows itself.

**The global shortcut** (`hotkey.rs`): registered once from `.setup()`
via `tauri_plugin_global_shortcut`, handled **entirely in Rust** — no
window's JS is involved in triggering a capture this way. A global
hotkey has no "requesting window" the way a button click does, and every
dependency the handler needs (`ActiveProcessState`, the same
`CaptureService::create_screenshot` §22 already built) is directly
reachable from Rust, so routing through IPC/a window's event loop would
have added a hop for no benefit. If there's no active Process, the
handler emits a `no_active_process` result instead of silently doing
nothing. **Registration failure is non-fatal** — logged, not propagated
with `?` — a deliberate fix made during this task after the originally-
chosen shortcut turned out to already be claimed by another application
on the verification machine and took the entire app down at startup
before the fix; see DECISIONS.md. No hotkey-customization UI exists —
the combination is a hardcoded constant (`hotkey::shortcut()`).

**Close-to-tray now applies to both windows.** TASK-010's
`on_window_event` handler (hide instead of close) was left as a single,
app-wide handler rather than branching by window label — on
reflection, both windows should behave the same way here: neither the
main window nor the widget should ever be truly destroyed while GoLive
is running, since either could be the user's only visible way back into
the app. (TASK-010's own docs had flagged this as something to revisit
once a second window existed — this is that reconsideration, landing on
"no branch needed" rather than the branch it anticipated.)

**Errors:** none new. The widget's own capture button surfaces failures
through the existing `getErrorMessage()` convention, same as
`CreateCaptureDialog`. The hotkey path (no window to show a normal error
UI in) converts an `AppError` to its safe `.to_string()` and carries it
in the `screenshot-captured` event's `error` variant instead.

**Testing:** 124 Rust tests (was 119) — 5 new: `active_process::tests`
(state starts empty, set/get round-trips, `None` clears a previous
value) and `hotkey::tests` (the `CaptureResult` enum's exact JSON tag
shape — pinning the wire contract `services/captures.ts`'s
`onScreenshotCaptured` parses — and that `shortcut()` is deterministic).
No new tests for `tray.rs`'s widget-toggle menu logic or the
event-emitting side of `sync_active_process` — both call real
native/Tauri APIs that (like the tray icon itself, TASK-010) can't be
constructed outside a running app; covered by manual verification
instead. UI-flow verification (main window: selecting/creating/renaming/
deleting a Process still calls `sync_active_process` with the correct,
now-fuller `{ process_id, process_name, project_id, project_name }`
shape; no regressions in any existing Project/Process/Capture flow) used
the same mocked-`window.__TAURI_INTERNALS__.invoke` dev-server approach
as every prior task. The widget page (`widget.html`) was separately
loaded in a browser tab and exercised directly — confirmed it renders
its "no active process" empty state correctly (capture button correctly
disabled) and that its hide button calls `hide_widget` — but the
"active Process shown → capture succeeds" path could not be directly
click-tested in this harness: the widget's initial fetch races the
mock's injection (there is no retry affordance to force a re-fetch after
the fact, unlike the main app's "Retry" button), and by the time a mock
can be installed the effect has already run and lost the race. This is a
testing-harness limitation, not a functional gap — both of that path's
constituent operations (reading the active-Process shape, calling
`createScreenshotCapture`) are exercised elsewhere (the main window's
regression pass; TASK-009's exhaustive screenshot verification).

**Native verification, not simulated:** the compiled `golive.exe` was
launched and, via a Win32 `EnumWindows` enumeration (since no desktop
UI-automation tool can click the native UI directly, the standing
limitation §22/§23 already record), both windows were confirmed to exist
and be visible simultaneously (`"GoLive"` and `"GoLive Capture"`). The
global shortcut was triggered for real — not simulated — via low-level
`keybd_event` Win32 input (not `SendKeys`, which only targets the
foreground window and would not reach a true OS-level global hotkey);
the app stayed running and responsive afterward, and (since no active
Process was set for that run) correctly created no capture, confirming
the `no_active_process` path executes without crashing. Close-to-tray
was re-verified with both windows present, the same graceful-close-
survives-as-same-PID method §23 used. See PROJECT_STATE.md for the full
record, including the startup-crash bug this verification pass actually
caught and how it was fixed.

## 25. Quick markers (TASK-012)

The near-zero-friction way to flag a moment during live
work without filling in a title/description — the "markers" README.md's
product description names as distinct from full notes. Deliberately the
smallest possible step: **no backend change at all.** TASK-008's generic
`create_capture` command/`CaptureService::create` already accepted an
arbitrary `title`/`description` for a `note`-type Capture; a marker is
simply that same operation called with the title/description decided by
the frontend instead of a human typing them into a dialog.

```
Widget's "Add marker" button
    ↓
Widget.tsx: handleMarker()
    ↓
createQuickMarker(processId)          features/projects/services/captures.ts
    ↓ builds { captureType: "note", title: "Marker — <formatDate(now)>", description: "" }
createCapture(...)                    the exact same call CreateCaptureDialog uses (TASK-008)
    ↓ invoke("create_capture")        — command/service/repository entirely unchanged
commands::capture::create_capture → CaptureService::create()
```

**`createQuickMarker`** (`features/projects/services/captures.ts`) is a
thin frontend-only wrapper: it generates the title as
`` `Marker — ${formatDate(Date.now())}` `` — reusing the exact same
`utils/formatDate` helper every other displayed timestamp in the app
already uses, so a marker's title reads the same way any other date text
in GoLive does — and an empty description, then calls the existing
`createCapture`. No new Tauri command, no new Rust code, no new
`AppError` variant, no migration. A marker is, from the moment it's
created, an ordinary `note`-type Capture: it appears in the existing
Captures list/detail like any other Note, and is edited (title,
description, even its type) through the existing generic Edit flow —
there is no special "marker" concept anywhere below the frontend
service layer, and no way to tell a marker apart from a hand-created
Note capture once it exists (by design — see DECISIONS.md).

**Surfaced only from the floating widget** (`src/widget/Widget.tsx`), as
a second button, "Add marker", next to TASK-011's "Capture screenshot" —
same disabled-when-no-active-Process rule, same inline success/error
feedback pattern, mutually exclusive with an in-flight screenshot
capture (one shared `busy` flag disables both buttons while either
request is in flight, to keep feedback from one action being overwritten
by the other). No global hotkey for markers — the roadmap step allowed
one optionally, but a second button already meets its "near-zero-
friction, one click, no dialog" goal without adding another
possibly-conflicting global shortcut registration (see §24's own
hotkey-collision story). The widget window (`tauri.conf.json`) grew
slightly taller (170px → 210px) to fit the second button without
crowding.

**Errors:** none new — a failed marker creation surfaces through the
same `getErrorMessage()`/inline-feedback convention the screenshot
button already used.

**Testing:** no new Rust tests — no Rust code changed; the full 124-test
suite continues to pass. No new frontend automated tests (this project
has no frontend test runner — see §2/README.md); verified instead by
directly invoking the compiled `createQuickMarker` module through the
Vite dev server with a mocked `window.__TAURI_INTERNALS__.invoke` (the
same technique every prior task's UI verification has used), confirming
it calls `create_capture` with exactly `{ process_id, capture_type:
"note", title: "Marker — <formatted timestamp>", description: "" }` and
correctly maps the mocked response back into a `Capture`. The widget's
"no active process" empty state was loaded directly (`widget.html`) and
confirmed to render both buttons, both correctly `disabled`. The
"active Process set → click Add marker → Capture appears" path could
not be click-tested end-to-end in this harness, for the identical
reason §24 already documents for the screenshot button (the widget's
on-mount fetch races mock injection with no retry affordance) — not a
new gap, the same one, now covering a second button. Given `createCapture`
itself is exhaustively covered (TASK-008's 16 service tests plus its own
full mocked-IPC UI pass) and this task added no backend surface for it
to call into, this is judged sufficient. The compiled `golive.exe` was
rebuilt and relaunched; both windows were confirmed to render at the new
widget size and the app started cleanly (no crash), matching §24's
native-verification standard.

## 26. Screen recording capture engine and storage (TASK-013)

Gives Recording Captures real video media, the same way
TASK-009 gave Screenshot Captures a real PNG — backend/native layer
first, UI deliberately minimal (a Start/Stop pair, no in-progress
indicator, no playback), matching PROJECT_STATE.md's own risk-reduction
flag for this feature ("highest risk — flagged for incremental
proof-of-concept development"). Full recording UX (in-progress
indicator reachable from the widget, video playback in `CaptureDetail`)
is TASK-014.

```
Two-phase flow (unlike screenshot's one-shot create_screenshot_capture):

CreateCaptureDialog, Type = Recording
    ↓ "Start recording" click
startRecordingCapture({ processId, title, description })
    ↓ invoke("start_recording_capture")
commands::recording::start_recording_capture
    ↓ CaptureService::validate_recording_start()   (shared with create/create_screenshot)
    ↓ generates the Capture id up front
    ↓ media.video_path(id)                          <captures_dir>/<id>.mp4
    ↓ WindowsRecordingEngine::start(path)            returns immediately
    ↓ stored in RecordingState (Tauri-managed, one recording at a time)
    ← RecordingStartedInfo { id, process_id, title }  (not a Capture — no row yet)

    ... real capture continues on windows_capture's own background thread ...

    ↓ "Stop recording" click
stopRecordingCapture()
    ↓ invoke("stop_recording_capture")
commands::recording::stop_recording_capture
    ↓ RecordingState::take()                         the stored handle + metadata
    ↓ handle.stop()                                   blocks until the MP4 is finalized
    ↓ CaptureService::finalize_recording(id, ...)     creates the Capture row, using
    ←  Capture (type: "recording")                    the SAME id the video was written under
```

**Native recording engine** (`native::recording`, mirroring
`native::screenshot`'s trait-object shape): `RecordingEngine::start(path)
-> Box<dyn RecordingHandle>` starts recording immediately and returns;
`RecordingHandle::stop(self: Box<Self>)` blocks until the file is fully
finalized and closed. `WindowsRecordingEngine` implements this using the
`windows-capture` crate (Windows Graphics Capture API +
hardware-accelerated Media Foundation video encoding) — **not** `xcap`'s
own `video_recorder()` (still an upstream work-in-progress at the time
this was written, and delivers only raw frames with no encoder attached)
and not a separately-chosen encoding crate/bundled ffmpeg binary paired
with it. `windows-capture` encodes directly to MP4 (H.264, default
settings) with no external runtime dependency — see DECISIONS.md for the
full comparison. Only the primary display is supported (`Monitor::
primary()`) — no monitor picker, no window/area selection, no
pause/resume, matching every scope limit TASK-009 already established
for the single-frame case.

**`media::MediaStorage` extended, not replaced** (see §7/TASK-009): PNG
methods (`save_capture`/`read_capture`/`delete_capture`/`exists`) are
untouched; new `video_path`/`video_exists`/`delete_video` methods are
their MP4 counterparts, with one structural difference — there's no
`save_video(bytes)`, because a recording's bytes are never fully in
memory at once. The recording engine writes directly to the path
`video_path` returns, incrementally, while the recording is in progress.
`reconcile` (the Project/Process-cascade orphan sweep, run once at
startup) now sweeps both `.png` and `.mp4` files — which incidentally
also cleans up a video file left behind by a recording that was never
stopped (e.g. the app was closed mid-recording): it has no matching
Capture row, so it's indistinguishable from any other orphan, and the
existing sweep removes it for free with no new code (see DECISIONS.md).

**Two-phase state lives in `recording::RecordingState`, not
`CaptureService`.** `CaptureService` is deliberately stateless — a fresh
instance is built for every command (`commands::recording::
capture_service`, matching `commands::capture::service`/`hotkey::
handle_capture_shortcut`'s already-established, independently-repeated
construction) — so an in-progress `RecordingHandle` can't live inside
it; it wouldn't survive between the `start` and `stop` commands, two
separate IPC calls. `RecordingState` is a small piece of Tauri-managed
state (`app.manage(...)`, same shape as `active_process::
ActiveProcessState`) holding at most one `InProgressRecording` at a
time — the one system-wide recording this task supports (see
DECISIONS.md). Its `start` method holds its internal lock for the
*entire* validate-and-launch sequence (not just the "is one already
running?" check), which is what actually prevents two concurrent
`start_recording_capture` calls from both starting a real native
recording before either notices the other.

**`CaptureService` gained two recording-specific methods,** alongside a
small refactor: `create`/`create_screenshot`'s shared
validate-title/description-and-confirm-process-exists logic was
extracted into a private `validate_for_create` helper (behavior
unchanged, now shared by three callers instead of duplicated twice).
`validate_recording_start` exposes that same validation to
`start_recording_capture` *before* a real recording begins — so an
invalid request never starts one that would then need to be aborted.
`finalize_recording(id, process_id, title, description)` is
`create_screenshot`'s mirror for the stop side: unlike every other
creation path, it takes an **externally supplied id** (the recording
engine already wrote `<id>.mp4` while the recording was in progress, so
the metadata row must reuse that id, not generate a fresh one),
confirms the video file actually exists (a defensive check — a
recording stopped the instant after starting could plausibly produce no
frames), and applies the same cleanup-on-metadata-failure discipline
`create_screenshot` established for PNGs. `delete` now removes both a
capture's PNG *and* MP4 unconditionally (both are safe no-ops when
absent, and a Capture is never both types) rather than just the PNG.

**Minimal UI, main window only (superseded by TASK-014 — see §27).** This
task originally put Start/Stop directly inside `CreateCaptureDialog`
(Type = Recording): title/description/type collected before the first
click started a real recording, the button then read "Stop recording"
with every other field locked until clicked. TASK-014 moved this to a
toolbar-level control in `CapturesSection` (and added one to the
widget) and removed "Recording" from the dialog's Type options
entirely — see §27 for the current shape; this paragraph is kept for
history, not as a description of current behavior.

**Errors:** none new — reuses `AppError::Capture` (native
start/stop/encode failures, "no video file was produced") and
`AppError::Validation` ("a recording is already in progress" / "no
recording is in progress").

**Testing:** 139 Rust tests (was 124) — 15 new: 5 `media::tests`
(`video_path`'s shape and UUID validation, `video_exists`, `delete_video`
incl. graceful no-op when missing, `reconcile` now removing orphaned MP4
files too), 5 `services::capture::tests` (`validate_recording_start`
sharing `create`'s rules, `finalize_recording` creating a Recording
Capture under the given id / failing cleanly when no video file exists /
cleaning up its video on a metadata-insert failure, `delete` removing
recording media alongside metadata), and 5 `recording::tests` against a
`FakeHandle` test double (`RecordingState::start` stores and returns
info / rejects a second concurrent start / doesn't store anything on a
failed start / a subsequent start succeeds after a failed one;
`RecordingState::take` returns once then `None`). No fake `RecordingEngine`
was needed in `services::capture`'s own tests — `finalize_recording`
never calls a `RecordingEngine` itself, only `MediaStorage`, so writing
a plain file to the expected path is enough to stand in for what the
real engine would have already produced.

Full UI-flow verification at the time (Projects → Process → Captures →
"+ New capture" → Type = Recording → title filled → "Start recording" →
confirmed via the mocked backend that `start_recording_capture` was
called with the exact expected shape, the dialog's title/description/
type fields and Cancel button all become `disabled`, and the button
relabels to "Stop recording" → clicked → confirmed `stop_recording_capture`
was called, the dialog closed, and the new Recording Capture appeared
in the list/detail pane exactly like any other capture-creation flow)
was verified against the Vite dev server with the same
mocked-`window.__TAURI_INTERNALS__.invoke` approach every prior task has
used, extended with `start_recording_capture`/`stop_recording_capture`
handlers (the mock also rejects a second concurrent start, mirroring the
real backend). **This dialog-based flow no longer exists** — TASK-014
(§27) replaced it with a toolbar-level control and re-verified the
equivalent flow there; this paragraph is a historical record of what
TASK-013 itself checked, not current behavior.

**Native verification, not simulated — the real risk this task existed
to de-risk:** `cargo test --release -- --ignored
record_primary_display_smoke_test` (the `#[ignore]`d real-capture test,
same convention as `native::screenshot`'s own smoke test) was run
explicitly on this session's real Windows desktop and **passed** — a
real ~2-second recording of the primary display produced a non-empty,
valid MP4. A further one-off spot-check
(`examples/recording_spotcheck.rs`, written temporarily and deleted
before this task was considered complete, with `lib.rs`'s `native`/
`errors` modules briefly widened to `pub` to reach them directly — same
temporary-widening convention TASK-004 through TASK-009 established)
recorded ~4 real seconds of this session's actual desktop to a
non-temp-dir path for direct inspection: the output was **156,431
bytes**, and its first bytes were verified byte-for-byte to be a
well-formed ISO-BMFF/MP4 box header (`00 00 00 18` = a 24-byte box
length, followed by the ASCII tag `ftyp`, followed by the major-brand
`mp42`) — this is definitive evidence of a genuine, correctly-encoded
MP4 container, not empty output or garbage bytes. The temporary example
file and the temporary `pub` widening were both removed immediately
afterward; `git status` was checked to confirm no stray files remained.

The freshly rebuilt `golive.exe` was also launched and, via the same
Win32 `EnumWindows` enumeration §22/§23/§24 established, both windows
were confirmed to render correctly and the app started with no crash.
What was **not** verified: an actual end-to-end "click Start recording
in the native UI → wait → click Stop → see the Capture in the list"
pass through the compiled app's own window, for the same
no-desktop-UI-automation limitation every prior task has recorded — the
mocked-IPC dev-server pass above proves the *frontend* wiring, and the
native smoke test above independently proves the *native engine*;
between the two, this is judged sufficient without overclaiming a
native-UI click-through that couldn't actually be performed here. A
developer with an interactive desktop session should still do one full
unassisted pass through `golive.exe` (start a recording, stop it,
confirm the video file at `%APPDATA%\com.golive.app\captures\<id>.mp4`
plays correctly in a real media player) before relying on this in a
real engagement.

## 27. Recording UI and playback (TASK-014)

**CURRENT.** Makes screen recording actually usable end to end — the
roadmap step's own words — building on TASK-013's backend/native
pipeline: a visible Start/Stop control reachable from two places, a
live in-progress/elapsed indicator that stays correct no matter which
window started the recording, and real video playback in
`CaptureDetail`.

```
Cross-window recording-status sync (mirrors §24's active-process sync):

commands::recording::start_recording_capture / stop_recording_capture
    ↓ update recording::RecordingState (unchanged mechanism, TASK-013)
    ↓ app.emit("recording-status-changed", Option<RecordingStatusInfo>)
every window's onRecordingStatusChanged(...) listener
    ↓
CapturesSection's toolbar control  ⇄  Widget's Start/Stop button
    both read the same status, whichever window started it
```

**Start/Stop moved out of the modal dialog** (see §26's superseded-
paragraph note): `CapturesSection` gained a toolbar-level `RecordingControl`
(sibling of "+ New capture"), and `Widget.tsx` gained a third button
alongside "Capture screenshot"/"Add marker". Both call the same
`startRecordingCapture`/`stopRecordingCapture` (`services/captures.ts`,
unchanged since TASK-013) with an auto-generated default title
(`defaultRecordingTitle()`, the same `formatDate`-based convention
`createQuickMarker` established, TASK-012) — no title dialog, matching
every other no-dialog quick-action in the app. `CreateCaptureDialog`'s
Type dropdown no longer offers "Recording" at all — a Recording Capture
is now created *only* through this dedicated flow, the same rule
Screenshot has followed since TASK-009 (see DECISIONS.md).

**Recording is system-wide** (§26/DECISIONS.md — one recording at a
time), so both controls handle three states, not two: no recording
anywhere (Start enabled), a recording in progress *for the Process this
control belongs to* (elapsed indicator + Stop), or a recording in
progress *for a different Process* (Start disabled with an explanatory
`title` tooltip, since starting a second one isn't possible — see
`recording::RecordingState::start`, unchanged since TASK-013).

**The widget itself has two in-window states** (a fix made immediately
after this task was first reported done — real use surfaced that the
window couldn't be dragged and didn't read as the small "dot" it was
meant to be; see DECISIONS.md, "TASK-014 bugfixes," for the full
story): collapsed to a small circular **dot** (`.widget-dot`, the
resting default) or **expanded** to the original panel with its
buttons. Clicking the dot expands it; the panel header's new "–" button
collapses back — distinct from "×", which still fully hides the OS
window via the unchanged `hide_widget`/tray flow. The window is
actually resized to match (`commands::widget::set_widget_expanded`,
`LogicalSize`), not just visually toggled with CSS inside one
fixed-size window, so the dot is a genuinely small window. Both states
carry `data-tauri-drag-region` so the widget can be dragged anywhere —
this requires `core:window:allow-start-dragging` in the widget's
capability file, which was missing entirely until this fix (the
attribute silently does nothing without it — see DECISIONS.md).
Windows enforces a minimum top-level window width (`SM_CXMIN`, ~130px)
regardless of what's requested, so a "56×56" dot actually becomes a
~133×56 window; `.widget-dot` is centered within whatever width the OS
actually grants (`position: fixed` + a centering transform) rather than
fighting the floor with native `WM_GETMINMAXINFO` interception.

**Cross-window sync** (`recording::RecordingStatusInfo`, `commands::
recording`'s `get_recording_status` command and `recording-status-changed`
event): the same shape §24 already built for the active Process —
`app.emit`, an event every window can `listen()` for, plus a
fetch-on-mount command for a window that wasn't open when the last
change happened. `RecordingStatusInfo` carries a raw `started_at` (epoch
ms); elapsed time is computed and ticked entirely client-side (see
below), never sent as a formatted string or pushed on an interval by the
backend. `stop_recording_capture` emits the "cleared" event *immediately*
after taking the in-progress recording out of `RecordingState` — before
calling `handle.stop()`/`finalize_recording`, both of which can still
fail — so no window is left showing a live ticking indicator for a
recording that isn't actually running anymore (see DECISIONS.md).

**`hooks/useElapsedSeconds`** — this project's first `hooks/` folder,
introduced because the elapsed-time ticking logic (`setInterval`
recomputing `Date.now() - startedAt` once a second, cleaned up on
unmount) is needed identically by two independent components (the
Captures section's toolbar and the widget) — crossing the "shared once
genuinely needed by multiple call sites" bar every other shared-code
decision in this project has used (see DECISIONS.md).
`utils/formatElapsed` renders the resulting whole-second count as
`M:SS`/`H:MM:SS`.

**Video playback** (`CaptureDetail`, Recording captures): a bounded
`<video controls>` element, the same "bounded, non-overflowing media
box" treatment §22 already established for the screenshot `<img>` —
both branches now share one media-fetching `useEffect`, differing only
in which service function they call. `getRecordingMediaUrl`
(`services/captures.ts`) is the Recording counterpart of
`getCaptureMediaUrl`: fetches the full MP4 via a new `get_recording_media`
command (mirrors `get_capture_media` — `tauri::ipc::Response`, Capture id
in, backend-derived file out, `AppError::NotFound` for a nonexistent
Capture or one with no video) and wraps the bytes in a `blob:` URL, with
the same caller-must-`URL.revokeObjectURL` contract. **Deliberately a
single in-memory byte-buffer transfer, not a streaming/range-request
protocol** (e.g. Tauri's `asset:` protocol) — evaluated and consciously
scoped down for this task rather than assumed; see DECISIONS.md for the
full trade-off (seeking still works over a `blob:` URL since the whole
file is already resident in memory once loaded; what's given up is
progressive/streaming playback start and a temporary doubling of memory
use for very large files — flagged, not yet measured, in
PROJECT_STATE.md's "Known technical risks"). **This player didn't
actually work at first** — `native::recording::WindowsRecordingEngine`
(§26) relied on `windows-capture`'s default video codec, which is HEVC,
and Chromium/WebView2 (what `<video>` runs on here) has no built-in HEVC
decoder; fixed by explicitly requesting H.264
(`VideoSettingsSubType::H264`) — see DECISIONS.md, "TASK-014 bugfixes,"
for the full story including how it was actually verified (scanning a
real recording's bytes for the codec's sample-entry FourCC, not just
re-checking the container header §26 had already checked).

**Errors:** none new — reuses the same `AppError` variants §26 already
covers; `get_recording_media` reuses `AppError::NotFound` exactly like
`get_capture_media`.

**Testing:** 144 Rust tests (was 139) — 5 new: `recording::tests::
status_reflects_the_current_in_progress_recording` (the new `status()`
accessor), 2 `media::tests` (`read_video` round-trips bytes / a missing
video returns `NotFound`, mirroring `read_capture`'s existing tests),
and 2 `services::capture::tests` (`get_recording_media` returns the
video bytes / returns `NotFound` for a capture with no video). No new
tests were needed for the `recording-status-changed` emit itself (same
reasoning §23/§24 already recorded for `tray.rs`'s widget-toggle logic
and `sync_active_process`'s own emit — real `AppHandle::emit` can't be
constructed outside a running Tauri app) — covered by the UI/native
verification below instead.

Full UI-flow verification (Projects → Process → Captures → toolbar's
"Start recording" clicked → confirmed via the mocked backend that
`start_recording_capture` was called with the exact expected
`{ process_id, title }` shape → the toolbar control switched to a live
"● Recording M:SS" indicator that was confirmed to actually tick across
a real multi-second wait (not just render once) → "Stop recording"
clicked → confirmed `stop_recording_capture` was called, the toolbar
returned to "Start recording", and the finished Recording Capture
appeared auto-selected in the list with a "Recording" badge → its
`CaptureDetail` was confirmed to render a real `<video>` element whose
`src` is a `blob:` URL built from the mocked `get_recording_media`
response, with no loading/error state left showing) was verified
against the Vite dev server with the same mocked-
`window.__TAURI_INTERNALS__.invoke` approach every prior task has used,
extended with a `transformCallback` stub so `@tauri-apps/api/event`'s
`listen()` doesn't throw in this harness (real event *delivery* still
can't be exercised here — the same standing limitation §24 already
documented, now also covering `recording-status-changed`; not a new
gap). The widget's structural layout (all three buttons present,
correctly `disabled` with no active Process) was confirmed the same way
§24/§25 verified their own buttons. The recording-specific service
functions the widget depends on (`defaultRecordingTitle`,
`startRecordingCapture`, `getRecordingStatus`) were additionally
verified directly via the same dynamic-`import()`-against-a-pre-mocked-
environment technique §25 used for `createQuickMarker`, confirming their
exact wire shapes — since `Widget.tsx`'s recording handlers are
structurally identical to `CapturesSection`'s (already exhaustively
click-tested above), this is judged sufficient without needing to win
the widget's own known-losing mount-race (§24) a third time.

**Native verification, not simulated:** the compiled `golive.exe` was
rebuilt and relaunched; both windows were confirmed to render with no
crash, the same Win32 `EnumWindows` check §22–§26 established.

**This task's first pass under-verified real playback — corrected
immediately after, see below.** The original claim here was "TASK-014
added no new native surface... so no further native smoke test was
needed," reasoning that §26/TASK-013's real-recording verification
(a genuine MP4 recorded from the desktop, header bytes confirmed
well-formed) already retired the native risk. That reasoning was wrong:
"the container is well-formed" and "the codec inside it actually plays
in Chromium/WebView2" are different claims, and only the first one was
ever checked. Real use immediately surfaced that recordings didn't play
at all — the encoder's default codec was HEVC, which Chromium doesn't
decode. Fixed by explicitly requesting H.264
(`VideoSettingsSubType::H264`, see DECISIONS.md, "TASK-014 bugfixes"),
and this time verified with a real recording whose raw bytes were
scanned for the codec's actual sample-entry FourCC (`avc1` present,
`hvc1`/`hev1` absent) — not just re-reading documentation or re-checking
the container header. The same pass also fixed two other real bugs
found by manual use, not assumed away: the widget wasn't draggable
(`data-tauri-drag-region` needs `core:window:allow-start-dragging`,
never granted) and didn't render as a small dot (Windows enforces a
~130px minimum window width regardless of what's requested) — both
covered in DECISIONS.md.

What was **not** verified even after this correction: an actual
end-to-end "click Start recording in the native UI → wait → click Stop
→ see the video play" pass through the compiled app's own window — the
same no-desktop-UI-automation limitation every prior task has recorded.
Given how directly this task's *first* "sufficient" claim already
turned out to be wrong, this gap is flagged more emphatically here than
usual: a developer with an interactive desktop session should do one
full unassisted pass (start a recording from the dot/widget while the
main window is hidden, stop it, open the main window, and confirm the
video actually plays in `CaptureDetail`) before relying on this in a
real engagement — the roadmap step's own definition of done, still not
independently confirmed end-to-end through the real native UI.

## Status

Reflects the state after **TASK-014 and its immediate bugfix pass**
(recording UI and playback — a Start/Stop control with a live
elapsed-time indicator is reachable from both the main window's
Captures section and the floating widget, whichever one started it, and
a finished Recording Capture actually plays back in `CaptureDetail` —
the codec bug that first broke playback entirely is fixed; the widget
is now a small draggable dot that expands to the full panel on click —
still no microphone audio, transcription, AI, or documentation-
generation functionality; Note Captures other than quick markers remain
metadata only). See [PROJECT_STATE.md](../PROJECT_STATE.md) for the
authoritative current implementation status.
