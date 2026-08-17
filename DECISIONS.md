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

## TASK-003 — Application shell and navigation

**Decision:** lightweight `useState`-based navigation in `App.tsx`
(`AppView` type + a `Record<AppView, () => ReactNode>` page map), no
routing library installed.
**Reason:** the shell has exactly two views and no requirement yet for
deep-linking, browser back/forward, or nested routes — a router would add
a dependency and API surface with nothing to justify it. The `PAGES` map
is deliberately shaped so it's a mechanical swap for real routes later
(see `docs/architecture.md` §16).
**Consequence:** adding a third page is a one-line addition to `NAV_ITEMS`
and `PAGES`; adopting `react-router` later means replacing the map with
`<Route>` elements and the `useState` with the router's location — no
change needed to `AppShell`, `Sidebar`, or `Header`.

**Decision:** application shell built from three layout components
(`AppShell`, `Sidebar`, `Header` in `components/layout/`) plus one reused
generic component (`EmptyState` in `components/ui/`); no other generic
`ui/` components were created.
**Reason:** these are the only pieces actually reused or structurally
necessary right now — `AppShell`/`Sidebar`/`Header` because the shell
itself needs them, `EmptyState` because both placeholder pages use it
identically. Nothing else in the shell repeats.
**Consequence:** `Projects`/`Settings` page-specific content (the "New
Project" button, copy) stays in each page file rather than being
abstracted into shared components it doesn't need.

**Decision:** introduced a small CSS design-token layer
(`src/styles/tokens.css`: color, spacing, radius, font) that `src/App.css`
builds on, instead of a CSS framework or CSS-in-JS library.
**Reason:** the shell needs consistent, reusable values (background,
surface, border, text, accent, spacing, radius) but not a full design
system; a plain CSS custom-properties file is zero-dependency and
sufficient at this scale.
**Consequence:** future UI should use the existing tokens rather than
hard-coding new colors/spacing; the `prefers-color-scheme: dark` variant
already established in TASK-001 was preserved by giving the same tokens
dark-mode overrides.

**Decision:** the Rust connectivity check (`check_foundation_status` via
`src/services/foundation.ts`) is now surfaced in the header as a status
dot + short label ("Ready" / "Connecting…" / "Offline") instead of the
previous standalone card; the raw backend message is kept only as a hover
tooltip.
**Reason:** the product requirement is a subtle, non-technical status
indicator ("Good: ● Connected. Avoid: check_foundation_status: true") —
mapping the three connectivity states to plain labels satisfies that
without hiding the underlying detail entirely (it's still available on
hover for debugging).
**Consequence:** no change to the underlying service or Rust command; the
proof-of-life mechanism from TASK-001 continues to work unmodified.

**Decision:** `tauri.conf.json` now sets `minWidth`/`minHeight` (760×480)
on the main window.
**Reason:** the two-pane shell (sidebar + content) needs a floor to stay
usable; the framework default has no minimum, so the window could
otherwise be resized small enough to make the sidebar or content
unreadable.
**Consequence:** users cannot resize the GoLive window below 760×480.

## TASK-004 — Local SQLite data layer

**Decision:** `rusqlite` (with the `bundled` feature) over `sqlx`.
**Reason:** `sqlx`'s main advantage — compile-time-checked queries — needs
either a live database or an offline query-cache file at build time,
adding real setup/maintenance overhead for every contributor; without
that feature we'd just be using its runtime-checked `query()` API, the
same thing `rusqlite` offers directly, while additionally pulling in an
async runtime (tokio) that nothing else in the app currently needs
(Tauri's commands don't require an async database driver — SQLite itself
is not an async database). `rusqlite` is synchronous, mature, has a much
smaller dependency tree, and `bundled` statically links SQLite into the
binary so users never install SQLite separately.
**Consequence:** database calls are synchronous Rust calls (via a pooled
connection, see below) rather than `.await`ed; this is the simpler,
lower-risk choice for a single-user desktop app and matches "simplest
robust solution" over "evaluate sqlx, don't default to it" from the task
brief.

**Decision:** connection pooling via `r2d2` + `r2d2_sqlite`, not a single
`Mutex<Connection>`.
**Reason:** a single shared connection behind a mutex would serialize
*every* database access — including reads — across the whole app. With
WAL mode (see below), SQLite already allows concurrent readers alongside
a writer; a pool lets the app actually take advantage of that once
concurrent work (recording, AI calls) exists alongside UI-driven reads,
without over-engineering a queue or actor system now.
**Consequence:** `db::DbService` hands out pooled connections
(`pool.get()`), not one shared connection; each pragma
(`foreign_keys`/`journal_mode`/`busy_timeout`) is applied via the pool's
`with_init` hook so every connection in the pool gets the same
configuration.

**Decision:** database location resolved via Tauri's
`app.path().app_data_dir()` (called once in `.setup()`), storing the file
at `<app_data_dir>/database/golive.db`. Never hardcoded.
**Reason:** required by the task brief and by the project's "no
hard-coded paths" principle; using Tauri's own resolver (rather than the
`dirs` crate or a manual `%APPDATA%` lookup) keeps the path correct for
the app's actual identifier and consistent with how Tauri itself resolves
paths for other purposes, without adding another crate that does the same
job.
**Consequence:** `db::DbService::init` takes a plain `&Path` rather than
resolving the directory itself, so it has zero Tauri dependency and is
fully unit-testable with `tempfile::tempdir()` — the Tauri-specific
resolution happens once, in `lib.rs`, and everything downstream is plain
Rust.

**Decision:** hand-rolled migrations (`PRAGMA user_version` + a fixed
`&[(version, sql)]` array), not the `rusqlite_migration` crate.
**Reason:** `rusqlite_migration` was evaluated and initially added, but
its latest version requires Rust 1.95 (this project's toolchain is
1.92.0), and the last version compatible with 1.92.0
(`rusqlite_migration@2.5.0`) depends on an older `rusqlite`/
`libsqlite3-sys` than the version already resolved for `r2d2_sqlite`,
which Cargo refuses to combine (two versions of a native library that
both `links = "sqlite3"` cannot coexist in one dependency graph). Rather
than downgrade `rusqlite`/`r2d2_sqlite` to work around a third-party
crate's transitive pin, the mechanism itself is small enough (SQLite's
own `user_version` pragma plus a version-ordered file list) to implement
directly in ~40 lines, with no dependency at all.
**Consequence:** one fewer dependency; migrations are still versioned,
source-controlled, applied automatically and in order, each inside its
own transaction, and safe to run repeatedly (only files newer than the
stored `user_version` execute). Revisit if a future task's migration
needs (e.g. down-migrations, checksums) outgrow this.

**Decision:** repository boundary is one trait
(`StorageStatusRepository`) with one method (`ensure_marker`) and one
implementation (`SqliteStorageStatusRepository`) — not a generic
`Repository<T>`.
**Reason:** explicitly required by the task brief, and consistent with
the project's standing rule against speculative abstraction: there is no
real domain model yet, so a generic repository would have nothing
concrete to be generic over.
**Consequence:** this is the reference shape for TASK-005's real
repositories (e.g. a future `ProjectRepository`), not a base class or
interface they need to extend.

**Decision:** implemented `AppError` (`thiserror`-derived, hand-written
`Serialize`) now, with `Storage` / `Database` / `Migration` variants only
— no `State` variant despite being one of the four categories the task
brief listed.
**Reason:** TASK-002 deferred a custom error type until a genuinely
fallible operation existed to justify it; the database layer is that
operation. A `State` variant was drafted but removed after `cargo check`
flagged it as dead code — nothing in the current codebase can actually
produce it (Tauri's `State<T>` extractor surfaces a missing/unmanaged
state as its own framework-level error, not one our code constructs), so
keeping an unused variant "for later" would repeat exactly the kind of
speculative code this project has consistently avoided.
**Consequence:** raw `rusqlite`/`r2d2`/`io::Error`s are converted at the
boundary (`impl From<...> for AppError`), logged to stderr for debugging,
and never reach the frontend as anything other than a fixed, generic
`{ code, message }`. Add `AppError::State` (or any other variant) the
moment a real code path needs it, not before.

**Decision:** all database tests use `tempfile::tempdir()`, never the
real per-user `app_data_dir`.
**Reason:** required by the task brief; also the only way to test
`init_is_idempotent_across_repeated_startups` and
`marker_survives_reopening_the_database` deterministically and
repeatably without depending on or mutating whatever happens to already
exist on the developer's machine.
**Consequence:** `cargo test` is always safe to run — it never touches
`%APPDATA%\com.golive.app`, and each test gets its own directory that's
deleted automatically when the test ends.

## TASK-005 — Project domain and persistence

**Decision:** Project id is a `Uuid::new_v4()` string, generated by
`ProjectService`, never accepted from the frontend.
**Reason:** required by the task brief; also keeps ids globally unique
and stable ahead of any future multi-machine/cloud sync, where
auto-increment integers assigned by a local SQLite `rowid` could collide
across two users' local databases.
**Consequence:** `uuid` (`v4` feature) added as a direct dependency (it
was already an indirect one via the toolchain). Frontend never sees or
assigns an id before a project exists server-side.

**Decision:** `created_at`/`updated_at` are Unix epoch **milliseconds**
(UTC), stored as SQLite `INTEGER`, generated by `ProjectService` — never
accepted from the frontend. Human-readable formatting happens only in the
frontend (`utils/formatDate.ts`), never stored.
**Reason:** matches the machine-representation convention TASK-004's
`storage_initialized_at` marker already established (there, Unix epoch
seconds); milliseconds instead of seconds for `Project` timestamps avoids
two projects created within the same second sharing an identical
`updated_at`, which would make sort order among them arbitrary. A
formatted string (e.g. ISO 8601) was considered and rejected: it would
require a date-formatting Rust dependency for no benefit, since the only
consumer that needs a formatted string is the frontend, which has
`Date`/`Intl` built in.
**Consequence:** `INTEGER` sorts correctly and cheaply for `ORDER BY
updated_at DESC` without a formatting/parsing step on either side of the
IPC boundary.

**Decision:** validation limits: name required, trimmed, ≤200 Unicode
scalar values; description optional, trimmed, ≤5000 Unicode scalar
values. Length counted in `chars()`, not bytes.
**Reason:** 200/5000 are reasonable ceilings for a project name and a
free-text description in a professional tool — generous enough not to
constrain real use, small enough to keep the UI and future exports
sane. Counting Unicode scalar values rather than bytes matters because
project content language is configurable per project (see the product
brief) — an Italian or other accented-character name shouldn't hit the
limit sooner than an English one of the same visible length just because
each accented character is 2 bytes in UTF-8.
**Consequence:** `ProjectService::create` is the single place these
limits are enforced (see `services::project::tests`); the frontend's
`maxLength` attributes on the create-dialog inputs are a UX convenience
only, not the source of truth.

**Decision:** default project order is `updated_at DESC` (most recently
touched first), backed by `idx_projects_updated_at`.
**Reason:** required by the task brief; matches how a consultant
actually works — the project they're currently engaged with is the one
they want at the top, not alphabetical order or creation order.
**Consequence:** `SqliteProjectRepository::list` always applies this
order; no sorting UI exists (or was requested) yet.

**Decision:** selected-project state lives as a plain `useState<string |
null>` inside `features/projects/ProjectsView.tsx`, not in a
`src/stores/` slice.
**Reason:** only the Projects feature currently reads or writes it —
promoting it to a shared store now would be exactly the kind of
speculative abstraction TASK-002 established the project should avoid.
**Consequence:** if a future feature (e.g. the floating widget, TASK-009+)
needs to know the current project, this becomes a same-shape
`stores/currentProject.ts` slice (`useState` → store hook, same value/
setter shape) — a swap, not a rewrite, of `ProjectsView` and whatever
consumes it next.

**Decision:** `update` is intentionally not implemented on
`ProjectRepository`/`ProjectService` in this task.
**Reason:** required by the task brief — TASK-005's UI has no edit form,
so an `update` method would be dead code. The trait/struct shape doesn't
need to change to add it later; it's one more method alongside
create/list/get/delete.
**Consequence:** `Project.updated_at` currently only reflects the
project's own `created_at` (set once, at creation) — it will start
diverging once TASK-006 adds editing.

**Decision:** deletion requires an explicit confirm dialog
(`DeleteProjectDialog`, "Delete project? ... This action cannot be
undone." / Cancel / Delete) and is hard, not soft (no "trash"/undo).
**Reason:** required by the task brief ("must require explicit
confirmation... do not allow accidental deletion from a single click");
soft-delete/trash and cascading cleanup of related entities (captures,
recordings, exports) has nothing to cascade to yet — no such tables
exist — so a hard delete of just the `projects` row is the simplest
correct behavior today, matching the "don't create cascading
relationships to tables that don't yet exist" instruction.
**Consequence:** a deleted project is unrecoverable through the UI. This
should be revisited once real project *contents* (captures, recordings)
exist and a delete has more to lose.

**Decision:** `rusqlite_migration` (added in TASK-004) is not reused —
migrations remain the hand-rolled `PRAGMA user_version` runner; the new
Project migration (`0002_projects.sql`) is simply appended to the same
fixed array.
**Reason:** no new information changed the TASK-004 evaluation; the
mechanism continued to work correctly for a second migration with zero
additional code.
**Consequence:** confirms the hand-rolled approach scales to multiple
migrations without friction; still revisit only if future needs (down-
migrations, checksums) genuinely outgrow it.

## TASK-006 — Project workspace and editing

**Decision:** Projects and the Project Workspace are two states of one
feature component (`ProjectsView` renders either the list or
`ProjectWorkspace` based on `activeProject`), not two top-level
`AppView`s or routes.
**Reason:** the top-level `Sidebar`/`AppView` navigation (TASK-003) is
for GoLive's main areas (Projects, Settings); "which project am I
looking at" is a concern entirely internal to the Projects feature, and
a routing library still isn't justified by one more internal state
switch — the exact reasoning TASK-003 already established for the
top-level nav applies unchanged here, one level deeper.
**Consequence:** entering/leaving the workspace is instant local state
(`setActiveProject`), no URL changes, no router dependency added; the
`WORKSPACE_TABS` array inside `ProjectWorkspace` is deliberately shaped
like the top-level `NAV_ITEMS`/`PAGES` pair so a future router swap (at
either level) is the same mechanical change.

**Decision:** `activeProject` is `useState<Project | null>` inside
`ProjectsView`, holding the full record (not just an id, as TASK-005's
`selectedId` did).
**Reason:** the workspace needs the whole `Project` to render immediately
(name, description, dates) without an extra `getProject` round trip on
every selection; TASK-005's `id`-only state was sufficient for a detail
pane fed by a `.find()` over an already-loaded list, but the workspace
model benefits from holding the object directly, especially since
`onUpdated` needs to replace it in place.
**Consequence:** still feature-local, still not a store — same
documented promotion path as before (§11) if a future feature needs the
current project.

**Decision:** the app-level `Header` continues to show "Projects" while
inside the workspace; it was not changed to show the project name.
**Reason:** the task explicitly allowed this ("if useful"). Making the
swap would require lifting `activeProject` out of `ProjectsView` into
`App.tsx` (the only component with access to `Header`), which breaks the
feature's self-containment for a purely cosmetic gain — the workspace's
own prominent name/description heading already identifies "what am I
looking at" without it.
**Consequence:** `Header`'s title continues to reflect only the top-level
area (Projects/Settings), unchanged since TASK-003; no new prop or
lifted state was introduced.

**Decision:** delete is available only from inside the Project Workspace
— not duplicated as a per-row action in the Projects list.
**Reason:** the task explicitly allowed either placement ("It is also
acceptable to keep deletion only in the Projects list if that fits the
current UI better" / workspace placement was the other named option);
with the list now a plain list (no more adjacent detail pane to hang a
Delete button off), and the "don't duplicate destructive actions
unnecessarily" instruction, putting it once, in the workspace (where the
user is already looking at the specific project), is simpler than adding
a second control.
**Consequence:** deleting a project always requires first opening its
workspace; `DeleteProjectDialog`'s confirm UX is otherwise unchanged from
TASK-005.

**Decision:** a dedicated `EditProjectDialog` (modal), not inline editing
within the workspace.
**Reason:** the task allowed either; a dialog reuses the existing
`components/ui/Dialog` shell and is structurally identical to
`CreateProjectDialog` (same two fields, same validation/error/loading
pattern), so it cost far less to build and keep consistent than an
inline edit-mode toggle for the workspace header would have.
**Consequence:** editing always opens a modal rather than editing the
header text in place; Cancel is "free" (local form state discarded on
unmount, no backend call), matching the spec's Cancel semantics exactly
with no extra code.

**Decision:** `AppError::NotFound` from `update_project`/`delete_project`
is handled specially by the frontend (`isNotFoundError()`,
`utils/errorMessage.ts`) rather than shown as a generic error.
**Reason:** required by the task brief ("If project no longer exists:
handle NotFound correctly; leave workspace gracefully; return user to
Projects"). Showing "The requested item could not be found." as an inline
form error when the real issue is "this project doesn't exist anymore"
would be confusing; exiting the workspace and dropping the stale entry is
the actually-correct response. For delete specifically, NotFound is
treated as an effective success — deleting an already-gone project ends
in the exact same state a successful delete would.
**Consequence:** `EditProjectDialog` takes an `onNotFound` callback
distinct from its generic error path; `DeleteProjectDialog`'s existing
`onDeleted` callback is reused for both "deleted" and "was already gone."

**Decision:** no new migration for `update` — the existing `projects`
table (from TASK-005's `0002_projects.sql`) already has every column
`update` needs.
**Reason:** required by the task brief ("Do NOT create a redundant
migration... Only create a migration if the actual existing schema
requires one"); confirmed by inspecting the schema before writing any
repository code.
**Consequence:** `ProjectRepository::update` is pure SQL against the
existing table; no schema change shipped with this task.

**Decision:** did not redesign the SQLite/persistence architecture in
response to TASK-005's reported real-`AppData` anomaly; a light
(non-exhaustive) real-`AppData` spot-check of the update path was
performed instead, and did not reproduce it.
**Reason:** required by the task brief; TASK-005 already thoroughly
investigated the anomaly (isolated it to something specific to the real
`%APPDATA%` path, most likely Windows real-time protection interfering
with a fresh WAL file from an unsigned dev binary — not a defect in
`rusqlite`/`r2d2`/the migration runner) and 44 deterministic automated
tests plus this task's own real-path spot-check both confirm the
mechanism itself is correct.
**Consequence:** no dependency, pooling, or migration-strategy changes
were made because of the anomaly; it remains documented in
PROJECT_STATE.md as an environment-specific observation, with a
recommendation to spot-check manually on a normal desktop session.
