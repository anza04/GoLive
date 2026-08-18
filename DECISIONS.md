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
exist and a delete has more to lose. (TASK-007 note: this is exactly
what happened — `processes` now cascades from `projects` at the database
level; see below.)

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

## TASK-007 — Process domain, persistence, and Workspace UI

**Decision:** `Process.project_id` is a real SQL foreign key
(`REFERENCES projects(id) ON DELETE CASCADE`), and deleting a Project's
Processes is left entirely to that constraint — no
`ProcessRepository::delete_by_project` and no loop in `ProjectService`.
**Reason:** required by the task brief ("Do not manually implement a
loop deleting processes from ProjectService. The database relationship
owns this behavior."); a database-level cascade is also atomic (can't
leave a project deleted with orphaned processes if the app crashes
mid-loop) and correct by construction rather than by an application
invariant someone has to remember to maintain as more child tables
appear later (Captures, Recordings, ...).
**Consequence:** `ProjectRepository`/`ProjectService` have zero knowledge
Processes exist — deleting a project is exactly the same one `DELETE FROM
projects WHERE id = ?` it always was. Proven by
`repositories::process::tests::deleting_a_project_cascades_to_its_processes`,
which creates two processes under one project, deletes the project via
`ProjectRepository`, and asserts both processes are gone. Requires
`PRAGMA foreign_keys = ON` on the deleting connection, which the pool
already sets on every connection (TASK-004) — this task didn't need to
add anything for the pragma itself, only the `ON DELETE CASCADE` clause
in the migration.

**Decision:** `ProcessStatus` is a real Rust enum with hand-written
`rusqlite::{ToSql, FromSql}` impls (not a bare `String` column with
validation only in the service), and derives `Serialize`/`Deserialize`
with `rename_all = "snake_case"` so the same three lowercase strings
(`draft`/`in_progress`/`completed`) are used in SQLite, in the Rust type,
and in the frontend's `ProcessStatus` TypeScript union.
**Reason:** required by the task brief ("represented as a Rust enum
rather than arbitrary strings inside business logic... Do not use
database integers for status"). Implementing `ToSql`/`FromSql` (rather
than converting to/from `String` manually at every repository call site)
keeps the repository's SQL binding/reading code exactly as simple as it
is for any other typed column, and makes an invalid stored value a
`rusqlite::Error` (mapped to the existing `AppError::Database`) instead
of a silent default or a panic.
**Consequence:** adding a status is a three-place, compiler-checked
change (the enum variant, `as_str`, `parse`) rather than a
string-matching bug waiting to happen; the frontend's `ProcessStatus`
union needs no separate mapping table for the wire values, only a
separate `STATUS_LABEL` map for *display* text (`ProcessStatusBadge`).

**Decision:** `ProcessService` holds both `Box<dyn ProcessRepository>`
and `Box<dyn ProjectRepository>`, and `create` explicitly verifies the
parent project exists before inserting.
**Reason:** required by the task brief ("When creating a Process: 1.
verify the Project exists 2. create the Process"). The foreign key alone
would already reject an insert against a nonexistent `project_id` — but
as a generic constraint-violation `rusqlite::Error`, which our
`From<rusqlite::Error> for AppError` maps to the generic `AppError::
Database` ("A local database operation failed."). The explicit check
gives the accurate, specific `AppError::NotFound` instead — the FK
remains a correctness backstop, the check is about error quality.
**Consequence:** `ProcessService::new` takes two repositories, unlike
every other service in this codebase so far, which take one — this is a
direct, justified consequence of Process being the first entity with a
required parent, not a pattern to generalize to other services without
the same reason.

**Decision:** default Process order is `updated_at DESC` within a
project (`idx_processes_project_id_updated_at`), matching Project's
existing convention exactly.
**Reason:** required by the task brief; same reasoning as TASK-005's
Project ordering — the process the consultant is actively documenting
should be at the top.
**Consequence:** `list_by_project` always applies this order; no sorting
UI exists.

**Decision:** Process selection state (`ProcessesView`'s `selectedId:
string | null`) stays a plain id, not promoted to holding the full
`Process` record the way Project's `activeProject` was in TASK-006.
**Reason:** Processes live inside an already-scoped Project Workspace
tab as a simple list+detail pane, not a second nested workspace with its
own back-navigation — the task explicitly asked to "keep the
architecture simple" and showed a list→detail (not list→full-page)
layout. A plain id plus `.find()` over the already-loaded list is
sufficient for a detail pane that's never unmounted/remounted the way a
whole-workspace swap would be.
**Consequence:** `ProcessesView` structurally mirrors TASK-005's
original `ProjectsView` (before TASK-006 introduced the workspace
concept for Projects), not TASK-006's current `ProjectsView` — a
deliberate, documented choice not to over-generalize the workspace
pattern to every parent/child relationship.

**Decision:** `list_processes` takes an explicit
`ListProcessesInput { project_id }` struct parameter instead of a bare
`project_id: String` command parameter.
**Reason:** Tauri's default argument handling camelCases bare command
parameter names for the JS-facing call (so a bare `project_id` parameter
would expect `invoke(cmd, { projectId })` from the frontend) — a
convention every other command in this codebase happens to never have
exercised, since every existing bare parameter (`id`) is a single word
with no case ambiguity. This project's UI verification method (a mocked
`window.__TAURI_INTERNALS__.invoke` in a browser preview) bypasses
Tauri's real argument extraction entirely, so it cannot independently
confirm that convention is applied correctly for a real build. Wrapping
the scalar in a struct removes the ambiguity outright: the top-level
parameter name (`input`) is single-word and case-invariant, and the
struct field (`project_id`) is matched by serde's literal field-name
deserialization, not Tauri's argument-name conversion — both sides now
depend only on mechanisms already proven correct by
`create_project`/`update_project`.
**Consequence:** one small `ListProcessesInput` struct instead of a bare
parameter; `list_processes` now has the same "wrap it in a struct" shape
as `create_process`/`update_process`, for consistency and to close a
real verification gap, not for its own sake.

## TASK-008 — Capture domain

**Decision:** `Capture` has no direct `project_id` — only `process_id`,
with `Project` reached transitively through `Process`. Deleting a Process
cascades to delete its Captures via a plain SQL foreign key
(`process_id ... REFERENCES processes(id) ON DELETE CASCADE`), and
deleting a Project cascades to Captures only as a side effect of two
chained foreign keys (`processes.project_id` → `projects`, then
`captures.process_id` → `processes`) — no `Capture.project_id` shortcut
column, and no application-side loop at either level.
**Reason:** required by the task brief's stated hierarchy ("Project 1 ───
N Process 1 ─── N Capture") and its explicit "Do not implement
application-side cascade loops" instruction. A denormalized
`Capture.project_id` would need to be kept in sync with the parent
Process's project (impossible to change today, but a needless invariant
to maintain the moment Processes can be reassigned) for a query
(project-wide captures) this task explicitly does not implement yet
(§13: "Do NOT create a project-wide capture list yet").
**Consequence:** `repositories::capture::tests::
deleting_a_project_cascades_through_processes_to_captures` proves the
transitive cascade end-to-end; `CaptureRepository`/`CaptureService` have
zero knowledge of Project at all, only Process — the same "each layer
only knows its direct parent" shape as `ProcessRepository`/
`ProcessService` not knowing about Project's siblings.

**Decision:** `Capture`'s Rust field is named `capture_type` (`type` is a
reserved word), with `#[serde(rename = "type")]` on the *model* so the
wire shape is `{ ..., "type": "screenshot", ... }` — matching the task's
documented `Capture { ..., type, ... }` shape and the frontend's
`Capture.type`. The Tauri command *input* structs
(`CreateCaptureInput`/`UpdateCaptureInput`), however, use the literal
field name `capture_type` with no rename, so their wire shape is
`{ ..., "capture_type": "screenshot", ... }` — exactly matching the task
brief's own explicit spec for those two structs.
**Reason:** the task brief specified the model's field as `type` but the
input structs' field as `capture_type` verbatim — not an oversight to
reconcile, but the two structs serving different purposes: the model is
what the frontend reads and needs to look like `Capture.type`; the input
structs are what the frontend writes, and Rust cannot name a plain field
`type` without raw-identifier syntax (`r#type`), which the brief's chosen
name `capture_type` sidesteps entirely for exactly the structs where it's
spelled out.
**Consequence:** `create_capture`/`update_capture` (Rust) and
`createCapture`/`updateCapture` (TypeScript) all use `captureType`/
`capture_type` consistently for writes; only the read path (`Capture`/
`get_capture`/`list_captures`) uses `type`. This asymmetry is deliberate
and documented here rather than "fixed" into one uniform name, since
uniforming it would contradict the task's own literal struct
definitions.

**Decision:** capture type validation happens in `CaptureService` against
a plain `&str` (`CaptureType::parse`, returning `AppError::Validation`
for anything unrecognized) — the Tauri command input structs
(`CreateCaptureInput.capture_type`, `UpdateCaptureInput.capture_type`)
are typed `String`, not `CaptureType`, even though `CaptureType`
implements `Deserialize`.
**Reason:** matches the existing `ProcessStatus` convention exactly
(`UpdateProcessInput.status: String`, TASK-007) — deserializing straight
into the enum would reject an invalid value with a generic Tauri
IPC-deserialization error, bypassing `AppError` entirely and violating
the task brief's explicit requirement that invalid capture types "reject
... through AppError::Validation."
**Consequence:** the same three tests this task added for it
(`create_rejects_invalid_capture_type`, `update_rejects_invalid_capture_type`,
plus the missing-process case) prove the rejection path returns a
structured, safe `AppError::Validation` message, not a raw deserialization
failure the frontend's `getErrorMessage()` can't handle.

## TASK-009 — Real screenshot capture

**Decision:** `xcap` (with its `image` feature) was chosen for native
screen capture, over hand-rolling GDI calls against the `windows` crate
directly, over `scrap`, and over `xcap`'s own predecessor crate
(`screenshots`, by the same author).
**Reason:** the task brief asked to "prefer an established Rust-
compatible Windows screenshot library" and avoid "a large framework."
`xcap` is actively maintained (unlike `screenshots`, which its own
maintainer has effectively superseded with `xcap`); its public API for
this task's exact need — list monitors, find the primary one, capture it,
get back an `image::RgbaImage` — is three calls
(`Monitor::all()`/`is_primary()`/`capture_image()`), and it re-exports
the `image` crate it already depends on (`xcap::image`), so no second
direct dependency was needed to encode PNG. `scrap` was not seriously
evaluated: it's lower-level (raw frame buffers, no built-in PNG path) and
less actively maintained than `xcap`. Hand-rolling GDI directly against
`windows-rs` was rejected as strictly more code and more Windows-API
surface for this project to own and maintain, for no capability `xcap`
doesn't already provide correctly.
**Consequence:** one new dependency (`xcap`), isolated entirely behind
`native::screenshot::ScreenshotEngine` (see docs/architecture.md §22) —
nothing above `WindowsScreenshotEngine` ever imports `xcap` directly.
`xcap`'s optional `wgc` feature (Windows Graphics Capture, extra
Direct3D/DXGI bindings) was deliberately left disabled: the default
GDI-based capture is the simplest option that reliably satisfies this
task's one required mode ("capture the primary/current display"), and
`wgc` would only be justified by a future need (e.g. capturing a
minimized/occluded window) TASK-009 doesn't have.

**Decision:** screenshot media is stored as a plain file
(`<app_data_dir>/captures/<capture-id>.png`) under the same
Tauri-resolved application-data directory the SQLite database already
lives in, keyed by `Capture.id` alone — no path column was added to the
`captures` table, and no database migration was needed.
**Reason:** required by the task brief almost verbatim ("Do NOT store
screenshot binary data inside SQLite," "prefer deriving the storage
location from Capture.id," "the existing Capture schema is intentionally
metadata-based"). Deriving the path from `id` also happens to be the
entire path-safety mechanism (see below) — no separate sanitization logic
was needed once the id itself is validated as a UUID.
**Consequence:** the Capture/media relationship is structural
(`captures/<id>.png`), not stored anywhere — `media::MediaStorage` is the
only code that ever constructs that path, and it does so identically for
every caller (create, read, delete, reconcile).

**Decision:** `MediaStorage::path_for` validates `capture_id` by parsing
it as a UUID (`uuid::Uuid::parse_str`) and rejects anything else with
`AppError::Validation`, rather than writing bespoke path-traversal
sanitization (stripping `..`, checking for absolute paths, canonicalizing
and verifying a prefix, etc.).
**Reason:** the task brief required that "path traversal... [be]
structurally impossible," not just filtered. A string that parses as a
UUID can, by definition, contain nothing but hex digits and hyphens in a
fixed 36-character shape — there is no character set overlap with `/`,
`\`, `:`, or `.` that any traversal or absolute-path attack needs, so
this single check is a strictly stronger, simpler guarantee than
denylisting dangerous substrings, and it doubles as validating that the
id is well-formed at all.
**Consequence:** `services::media::tests::paths_cannot_escape_the_media_
directory` (in `media::tests`) proves several classic traversal payloads
(`../../../etc/passwd`, a Windows UNC-style `..\..\windows\system32`, an
absolute path, an embedded `/`) are all rejected identically, and that
nothing is ever written to disk for any of them.

**Decision:** screenshot creation is orchestrated at the service level
(capture → save PNG → insert metadata row, with the PNG deleted if the
metadata insert fails) rather than as a real database transaction.
**Reason:** required by the task brief, which explicitly said a real DB
transaction isn't the right tool here ("the exact implementation can use
a service-level orchestration rather than a database transaction because
the media is filesystem data"). SQLite transactions can't span a
filesystem write; wrapping only the DB insert in one wouldn't have
changed anything about the actual risk (an orphan file after a DB
failure), which is what needed handling.
**Consequence:** `CaptureService::create_screenshot` writes the PNG
*before* the metadata row specifically so a metadata-insert failure has
something to clean up — the reverse order would risk the opposite,
strictly worse failure mode (a Capture row implying media that was never
written). Proven by `create_screenshot_cleans_up_the_png_when_metadata_
insert_fails`, using a `CaptureRepository` test double whose `create`
always fails.

**Decision:** orphaned screenshot media left behind by a Project/Process
cascade delete is cleaned up by a reconciliation sweep at application
startup (`CaptureService::reconcile_media`, called once from `lib.rs`'s
`.setup()`), not synchronously during the cascade itself. Direct Capture
deletion (`CaptureService::delete`) still cleans up its media
immediately.
**Reason:** the task brief was explicit that this was an acceptable,
even preferred, outcome if the alternative compromised existing
boundaries: "Do not introduce an inefficient: for every capture: delete
file loop inside Project deletion... If full automatic media cleanup for
cascaded database deletions cannot be implemented safely... without
compromising the existing cascade architecture, document the limitation
clearly." Making `ProcessService`/`ProjectService` aware of
`CaptureRepository`/`MediaStorage` so they could delete files
synchronously during their own delete calls would have inverted the
domain's dependency direction (currently strictly Capture → Process →
Project) for every future child domain, not just this one, and turned
"delete a project" into an operation that also has to walk every
descendant's media — exactly the "loop... without considering the
existing repository and service boundaries" the brief warned against.
**Consequence:** a PNG orphaned by a cascade delete is not removed until
GoLive is next launched, not the instant the cascade happens — stated
explicitly in docs/architecture.md §22 as a known, deliberate limitation
rather than something silently swept under the rug. Reconciliation itself
is cheap (one directory listing, one `SELECT id FROM captures`, one set
difference) and generic — the same sweep will clean up orphaned recording
files whenever that media type exists, with no changes needed.

**Decision:** editing a screenshot Capture's `type` away from
`screenshot` (via the existing generic edit dialog/command) does not
delete, move, or otherwise touch its PNG file. The file simply becomes
unreachable through the UI (which only requests media for
`type === "screenshot"`) until/unless the type is changed back.
**Reason:** the task brief explicitly flagged this exact scenario as
something to think through rather than hack around: "Do not accidentally
create impossible states such as: type changed from screenshot to note
while an orphan screenshot PNG remains... preserve existing functionality
and document the decision rather than silently deleting or replacing
media." Deleting the file on a type-away edit would be a destructive,
surprising side effect of what the user experiences as an ordinary
metadata edit (Edit dialog, Save) with no explicit "delete media" action
taken; deleting it and finding out `update` can never recreate it (that's
`create_screenshot`'s job, not `update`'s) would make changing the type
back to Screenshot silently show nothing, which is worse than doing
nothing at all.
**Consequence:** because the file is keyed by `Capture.id` (not by
`capture_type`), this "do nothing" choice has a pleasant side effect for
free: changing the type back to Screenshot later makes the original
image reappear correctly, with zero special-case code — proven by
`update_changing_type_away_from_screenshot_does_not_delete_media`.

**Decision:** one new `AppError` variant, `Capture(String)`, was added
for native capture-engine failures (no display available, PNG encode
failure) — `xcap::XCapError` converts to it via a `From` impl.
Filesystem failures actually *storing* media (directory creation, PNG
read/write/delete) deliberately reuse the existing `AppError::Storage`
(via the existing `From<std::io::Error>` impl) instead of getting their
own variant.
**Reason:** the task brief permitted a new variant if none of the
existing ones were genuinely appropriate ("If existing Storage/Database
errors are appropriate, reuse them... Do not introduce new AppError
variants unless genuinely necessary"). None of `Storage`/`Database`/
`Validation`/`NotFound` accurately describes "the OS couldn't capture the
screen" — it's not a storage-preparation problem, not a SQL problem, not
malformed user input, and not a missing record — so a new variant was
judged genuinely necessary there. A filesystem failure *storing* the PNG,
by contrast, is exactly the same category `Storage`'s existing doc
comment already covers ("the local application-data directory or
database file could not be prepared or accessed") once that comment was
extended to also name the captures directory — reusing it needed no new
code at all, since `MediaStorage`'s `std::fs` calls already convert via
the pre-existing `From<std::io::Error>` impl.
**Consequence:** `AppError::Capture` carries the same "author-written,
safe, shown as-is" contract as `Validation` (see §10) — e.g. "No display
is available to capture." / "Screenshot capture failed. Please try
again." — never a raw `XCapError`/`ImageError` message. Its `code` is
`"capture_error"`, a new stable string the frontend can (but doesn't yet
need to) branch on, the same way `isNotFoundError()` branches on
`"not_found"`.

**Decision:** `get_capture_media` returns `tauri::ipc::Response` (raw
bytes) instead of a JSON-serialized `Vec<u8>` or a Tauri asset-protocol/
`convertFileSrc` URL exposing a real filesystem path.
**Reason:** the task brief explicitly forbade a `read_file(path)`/
`get_file(path)`-shaped command "where the frontend can provide arbitrary
filesystem paths," and asked for "the simplest approach compatible with
the existing Tauri 2 architecture and security model." A raw-bytes IPC
response keyed only by Capture id satisfies both: no path ever crosses
the IPC boundary in either direction, and `tauri::ipc::Response` is
Tauri 2's own documented mechanism for this exact "return binary data
without the ~3-4x JSON-array size/parse overhead" problem — no new Tauri
capability/scope was needed, since it's IPC, not the asset protocol.
**Consequence:** `services/captures.ts`'s `getCaptureMediaUrl` receives
the bytes as an `ArrayBuffer` (Tauri's `invoke()` detects a `Response`-
returning command automatically) and wraps them in a `blob:` object URL
for `<img src>` — the frontend never sees, stores, or could construct a
real filesystem path for a Capture's media.
