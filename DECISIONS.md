# DECISIONS

Architectural and product decisions, in chronological order. Each entry
records what was decided and why.

## TASK-001 — Project foundation

**Tauri 2** as the desktop application shell.
Why: purpose-built for lightweight, secure, native-feeling desktop apps
with a small binary/memory footprint compared to Electron; first-class
Windows support; a Rust backend gives direct access to native Windows APIs
that later milestones (screen recording, global hotkeys, Credential
Manager) will require.

**React + TypeScript** for the frontend.
Why: mainstream, well-supported, large ecosystem, strong typing reduces
runtime errors in a UI that will grow to cover projects/captures/processes;
team familiarity.

**Rust** for the backend/native layer.
Why: required by Tauri; also the right tool for the performance- and
reliability-sensitive native work ahead (screen/audio capture, global
hotkeys, filesystem/SQLite access) without a garbage collector or another
runtime in the mix.

**Windows-first MVP.**
Why: set by product requirements (section 2 of the project brief); the
`tauri.conf.json` bundle target is scoped to `nsis` (Windows installer)
rather than `"all"`, since building for platforms the MVP does not target
would be wasted effort and could hide Windows-specific issues.

**npm** as the package manager.
Why: no existing repository convention to follow; npm is the default for
`create-tauri-app` and requires no extra tooling.

**No Tauri plugins beyond the framework defaults.**
Why: the scaffolded `tauri-plugin-opener` (for opening external links) was
removed since the foundation shell has no external links to open —
principle of least privilege. Plugins will be added only when a task
actually needs their capability.

**No Rust module subfolders (`commands/`, `services/`, `repositories/`,
`models/`) created yet.**
Why: creating empty/near-empty modules now would be speculative structure
with no content to justify it. `src-tauri/src` currently holds only
`main.rs` and `lib.rs` with a single proof-of-life command. These modules
will be introduced when the first real Tauri commands and business logic
land (starting TASK-002 / M1).

**Demo `greet` command replaced with `check_foundation_status`.**
Why: the template's `greet` command exists only to prove the
frontend-to-backend pipeline works; renaming it to a health-check with no
required input keeps that verification purpose explicit and avoids
resembling actual product functionality.

## TASK-002 — Architecture hardening and development conventions

**Decision:** React components call a frontend service function, never
`invoke()` directly. App-level service functions live in `src/services/`;
feature-specific ones will live in `features/<feature>/services/`.
**Reason:** keeps every native/Tauri touchpoint discoverable and
consistent as the number of commands grows; prevents `invoke()` calls from
being scattered across arbitrary components.
**Consequence:** `App.tsx` now calls `checkFoundationStatus()` from
`src/services/foundation.ts` instead of calling `invoke()` inline — the
only code change this task makes to existing behavior (output is
unchanged). Every future Tauri call must follow the same pattern.

**Decision:** no custom Rust error type (`AppError`) is implemented yet.
**Reason:** the only existing command, `check_foundation_status`, is
infallible — it has no failure path to model, so a custom error type would
be speculative structure with nothing real behind it.
**Consequence:** the intended convention (`Result<T, AppError>` with a
`code` + `message`, `thiserror`-based, no panics for expected failures) is
documented in `docs/architecture.md` §10 and must be implemented starting
with the first command that can genuinely fail (expected in M1/M2).

**Decision:** no Rust `commands/` / `services/` / `repositories/` /
`models/` / `errors/` module split yet; `src-tauri/src` stays as
`main.rs` + `lib.rs`.
**Reason:** one command with no branching does not justify five empty
modules. Splitting now would be structure without content.
**Consequence:** module boundaries and what belongs in each are documented
in `docs/architecture.md` §3, to be created as real code lands in each.

**Decision:** no frontend state-management library and no routing library
are adopted.
**Reason:** the application has exactly one view and one piece of local
state (`useState` in `App.tsx`); nothing today needs shared state or
navigation.
**Consequence:** local component state remains the default; a `stores/`
entry is added only once at least two features need to share state; a
routing library is introduced only once multiple pages actually exist
(expected starting TASK-003, "Application shell and navigation").

**Decision:** persistence, file storage, and AI are each defined as a
layered abstraction (domain/feature logic → provider-agnostic interface →
concrete implementation) but none is implemented in this task.
**Reason:** TASK-002 is documentation/hardening only; implementing any of
SQLite, filesystem storage, or an AI provider now would be product
functionality out of scope for this task and would risk locking in
decisions (schema, storage layout, provider client shape) before the tasks
that actually need them are scoped.
**Consequence:** `docs/architecture.md` §6–§9 record the intended
boundary for each so future tasks implement behind the right seam from the
start, without this task writing any SQLite/filesystem/OpenAI code.

**Decision:** Tauri capabilities remain limited to `core:default`; no new
permissions requested.
**Reason:** nothing implemented so far needs filesystem, shell, global
shortcuts, microphone, screen capture, or network access — granting them
now would violate least privilege ahead of need.
**Consequence:** each future capability is added only in the task that
implements the feature requiring it.

**Decision:** `serde` and `serde_json` are kept as direct Rust
dependencies rather than removed.
**Reason:** although no custom type in this repo uses them directly yet,
they are already a `tauri` transitive dependency, are near-certain to be
needed the moment any command exchanges a struct (`#[derive(Serialize,
Deserialize)]`) or the AI integration handles JSON, and are standard,
low-risk, actively maintained crates — removing them now would very likely
mean re-adding them in the next task or two.
**Consequence:** no dependency change; documented in
`docs/architecture.md` §15 as "present ahead of imminent, not speculative,
need."

**Decision:** added `.env` / `.env.*` (except `.env.example`) to
`.gitignore`.
**Reason:** defensive guard against accidentally committing secrets,
consistent with the project's "no secrets in source control" rule, even
though no `.env`-based configuration exists yet.
**Consequence:** a one-line `.gitignore` addition; no `.env` file was
created.
