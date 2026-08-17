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

**CURRENT:** `src-tauri/src` contains only `main.rs` (entry point) and
`lib.rs` (Tauri builder + the single `check_foundation_status` command).
That's an accurate reflection of the actual functionality: one command with
no branching, no fallible operations, no persistence. Adding empty
`commands/`, `services/`, `repositories/`, `models/`, `errors/` modules now
would be structure with nothing to hold.

**FUTURE — where new Rust code belongs, as it's introduced:**

- **`commands/`** — Tauri-facing functions (`#[tauri::command]`). Thin:
  parse/validate the call, delegate to a service, map the result/error back
  to the frontend. No business logic here.
- **`services/`** — Application/business logic. Pure Rust, independently
  testable, no direct knowledge of Tauri's `invoke` plumbing.
- **`repositories/`** — Persistence abstractions (see §6). The future
  SQLite implementation lives behind this boundary.
- **`models/`** — Domain/data structures shared across backend logic
  (`Project`, `Process`, `Capture`, ...).
- **`errors/`** — Application-level error type(s) (see §10).

Introduce each module when the first real code needs it — e.g.
`commands/` and `services/` most likely arrive together with the first
Project command in M1, `repositories/` and `models/` with SQLite in M2.

## 4. Frontend → Tauri communication

**Convention (CURRENT, established by this task):**

```
React component
    ↓
frontend service function      src/services/*.ts
    ↓                          or features/<feature>/services/*.ts
Tauri invoke()
    ↓
Tauri command                  src-tauri/src (commands/ once it exists)
    ↓
application service            [FUTURE]
    ↓
repository / native service     [FUTURE]
```

**Rule:** React components never call `invoke()` directly. They call a
service function, which is the only place `invoke()` appears.

**Where the service function lives:**
- Not tied to a specific feature → `src/services/`.
- Belongs to one feature → `features/<feature>/services/`.

**Concrete example (the only one that exists today):** `App.tsx` calls
`checkFoundationStatus()` from [`src/services/foundation.ts`](../src/services/foundation.ts),
which calls `invoke("check_foundation_status")`, which reaches the Rust
command in [`src-tauri/src/lib.rs`](../src-tauri/src/lib.rs). No pointless
extra layers (no application-service/repository step) were added around
this single infallible command — those layers are introduced when a real
service/repository exists to justify them, not before.

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

**FUTURE.** The MVP will use SQLite, but it is **not** introduced by this
task.

```
Application/domain logic (Rust)
        ↓
Repository interface (Rust trait)
        ↓
SQLite implementation
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

**CURRENT:** the only existing command, `check_foundation_status`, cannot
fail — it returns an owned `String` directly, no `Result`, no panic path.
No custom error type exists yet, and introducing one now would have nothing
real to model.

**FUTURE convention**, to apply once the first fallible command is added:
- Tauri commands that can fail return `Result<T, AppError>`, where
  `AppError` is a small `serde::Serialize`-able error type (e.g. built with
  `thiserror`) carrying a stable `code` and a human-readable `message`.
- Expected failure conditions (not-found, validation, I/O failure, etc.)
  are returned as `Err(...)`, never `panic!`/`unwrap`/`expect`. `.expect()`
  remains acceptable only for genuinely unrecoverable startup failures,
  exactly as `run(...).expect(...)` is used today in `lib.rs` for a fatal
  application-bootstrap error.
- The frontend service layer (§4) receives this structured error via the
  rejected `invoke()` promise and turns it into a message a user can
  understand, while keeping the technical detail (`code`, underlying
  message) available for debugging/logs.

This is documented rather than implemented now because no fallible
operation exists yet to justify the type (see decision in DECISIONS.md).

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
| `serde` (derive) | (De)serialization traits — not yet used by any of our own types, but required by essentially every future Tauri command that exchanges structured data (e.g. `#[derive(Serialize, Deserialize)]` domain structs) and already a transitive dependency of `tauri` itself. Kept rather than removed: standard, low-risk, and imminently needed starting with the first typed command. |
| `serde_json` | JSON value handling — same rationale as `serde`; will be used once structured AI responses or JSON payloads exist. |

No dependency was added or removed by this task; `tauri-plugin-opener` was
already removed in TASK-001 and is not reintroduced.

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
  flow" rule.

## Status

Reflects the state after **TASK-003** (application shell and navigation —
no product functionality added). See [PROJECT_STATE.md](../PROJECT_STATE.md)
for the authoritative current implementation status.
