# PROJECT_STATE

Project:
GoLive

Current milestone:
M2 — AI Structuring (see roadmap.md; M1 — Live Capture completed at
TASK-015 — every capture modality the product promises now works)

Completed:
TASK-001, TASK-002, TASK-003, TASK-004, TASK-005, TASK-006, TASK-007,
TASK-008, TASK-009, TASK-010, TASK-011, TASK-012, TASK-013, TASK-014,
TASK-015

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
- **Background persistence, system tray, and the active-process store
  (TASK-010)** — the first step of milestone M1 (see roadmap.md):
  - `tray.rs` (new top-level module) + `commands/tray.rs`: builds a
    system tray icon (Tauri's built-in `tray-icon` Cargo feature, no new
    dependency) with a three-item menu — a disabled, informational
    "Active: …"/"No active process" label, "Open GoLive", "Quit"
  - `lib.rs`'s `Builder::on_window_event` intercepts `WindowEvent::
    CloseRequested` and hides the window instead of closing it — GoLive
    now keeps running with its main window hidden; the tray's "Quit" is
    the only real exit besides a window-manager force-close. Manually
    verified against the compiled binary: a graceful close request left
    the same process running; force-close still works
  - `tauri.conf.json`'s window now has an explicit `"label": "main"`
  - `src/stores/activeProcess.tsx` — `src/stores/`'s first real
    occupant: a plain React Context (`ActiveProcessProvider`/
    `useActiveProcess`, mounted once in `App.tsx`), not a state
    library, holding which Process is currently "active." Purely
    client-side — no backend model, nothing persisted, resets on
    every launch
  - `src/services/activeProcess.ts`'s `syncActiveProcess` (renamed/
    widened by TASK-011 — see below; was `services/tray.ts`'s
    `syncActiveProcessTray`) keeps the tray in sync with the store as a
    side effect of `setActiveProcess`/`clearActiveProcess`, so callers
    never call the Tauri command directly
  - `ProcessesView.tsx`: selecting, creating, or renaming-while-active a
    Process marks/updates it as active (implicit — no separate "Set
    active" action); deleting the active Process clears it.
    `ProjectsView.tsx`: deleting a Project clears the active Process if
    it belonged to that Project
  - The underlying command (`commands::tray::set_active_process_tray` at
    the time) was intentionally infallible (no `AppError`) — a tray-label
    update is a cosmetic side effect the frontend never needs to react to
    failing; native failures are logged server-side only. Still true of
    its TASK-011 successor, `sync_active_process`
- **Global hotkey and floating capture widget (TASK-011)** — the first
  step that actually delivers "capture while performing your actual
  work": a Screenshot can now be captured from anywhere in Windows, into
  the active Process, without switching to GoLive's main window:
  - Second Tauri window (`tauri.conf.json`, label `"widget"`,
    `url: "widget.html"` — a second Vite/HTML entry, `vite.config.ts`'s
    `build.rollupOptions.input`): small, always-on-top, no decorations,
    hidden from the taskbar, visible by default. Its own React tree
    (`src/widget/Widget.tsx`, `widget-main.tsx`, `widget.css`) — a new
    top-level `src/widget/` folder, sibling to `features/`, not a
    feature of the main app
  - `active_process.rs` (new top-level module): `ActiveProcessState`, a
    small `Mutex<Option<ActiveProcessInfo>>` — the cross-window
    active-Process mirror. Tauri windows share no JS memory, so the
    widget can't read the main window's `stores/activeProcess.tsx`
    Context directly; this Rust-side state (updated by the renamed/
    widened `sync_active_process` command, now carrying full
    `process_id`/`project_id`, not just display names) plus an
    `active-process-changed` event broadcast to every window
    (`AppHandle::emit`, Rust-side — not ACL-gated) is how the widget
    stays in sync. `get_active_process` lets a window fetch the current
    value on mount. Never persisted — resets every launch
  - `hotkey.rs` (new top-level module) + `tauri-plugin-global-shortcut`
    (new dependency, Tauri's own first-party plugin, Rust-API-only — no
    JS package added): registers `Ctrl+Alt+Shift+S` in `.setup()` and
    handles it **entirely in Rust** — reads the active Process, builds a
    `CaptureService` the same way `commands::capture` does, calls the
    exact same `create_screenshot` (§22, unchanged), and emits a
    `screenshot-captured` result event. No window's JS is involved in
    triggering the capture this way; if there's no active Process, the
    handler says so via the event instead of silently no-op'ing
  - **Startup-crash bug found and fixed during this task's own manual
    verification:** the first version propagated a shortcut-registration
    failure with `?`, which Tauri treats as a fatal `.setup()` error —
    when the originally-chosen shortcut turned out to already be claimed
    by another application on the verification machine, GoLive crashed
    on every launch. Fixed: registration failure is now logged and
    non-fatal (the app starts normally with the hotkey simply inactive);
    the default shortcut was also changed to a less collision-prone
    three-modifier combination as a secondary mitigation
  - `tray.rs` gained a fourth menu item, "Toggle Widget" (shows/hides
    the widget window — Rust-only, no capability needed)
  - `commands::widget::hide_widget`: the widget's own "×" button calls
    this dedicated command rather than the raw `@tauri-apps/api/window`
    API directly — sidesteps needing `core:window:allow-hide` (not
    included in `core:default`) in its capability entirely, since
    app-defined commands aren't ACL-gated
  - New capability file `capabilities/widget.json` (scoped to the
    `"widget"` window only): `core:default` plus
    `core:event:allow-listen` — the one permission the widget genuinely
    needs, to receive the events above via `@tauri-apps/api/event`'s
    `listen()` (first use of that module; wrapped in thin service
    functions, `onActiveProcessChanged`/`onScreenshotCaptured`, same
    "components never call a raw Tauri API directly" rule as `invoke`)
  - `lib.rs`'s close-to-tray handler (TASK-010) was reconsidered, not
    changed: both "main" and "widget" hiding instead of closing is
    correct for both, so no per-window branching was needed after all
- **Quick markers (TASK-012)** — the near-zero-friction way to flag a
  moment during live work, with **no backend change of any kind**:
  - `createQuickMarker(processId)` (new, `features/projects/services/
    captures.ts`): a thin frontend-only wrapper around the existing,
    unmodified `createCapture` — calls it with `captureType: "note"`, a
    title auto-generated as `` `Marker — ${formatDate(Date.now())}` ``
    (reusing `utils/formatDate`, the same helper every other displayed
    timestamp in the app uses), and an empty description. No new Tauri
    command, no new `AppError` variant, no migration — TASK-008's
    generic `create_capture` already accepted exactly this shape
  - `Widget.tsx` gained a second button, "Add marker", next to TASK-011's
    "Capture screenshot" — same disabled-when-no-active-Process rule,
    same inline success/error feedback. Both buttons now share one
    `busy` flag (`capturing || marking`) so only one capture action runs
    at a time and the single shared feedback line is never overwritten
    mid-flight by the other action
  - `tauri.conf.json`'s widget window grew from 170px to 210px tall to
    fit the second button without crowding
  - Once created, a marker is an entirely ordinary `note`-type Capture —
    same list/detail rendering, same generic Edit dialog, no special
    badge, no way to distinguish it from a hand-typed Note after the
    fact (a deliberate choice — see DECISIONS.md)
  - No global hotkey was added for markers, even though the roadmap step
    allowed one optionally — the widget button alone already satisfies
    "one click, no dialog," and TASK-011 already showed that every
    additional global shortcut GoLive registers is a real chance to
    collide with another application (see DECISIONS.md)
- **Screen recording capture engine and storage (TASK-013)** — a
  Recording Capture now has a real, playable MP4 file behind it, the
  same way TASK-009 gave Screenshot Captures a real PNG. Backend/native
  layer only — UI is deliberately minimal (a Start/Stop pair, no
  in-progress indicator, no playback); full recording UX is TASK-014:
  - `native::recording` (new): `RecordingEngine`/`RecordingHandle`
    traits (mirroring `ScreenshotEngine`'s shape) + `WindowsRecordingEngine`,
    using the new `windows-capture` crate (Windows Graphics Capture API
    + its own hardware-accelerated Media Foundation MP4 encoder — no
    bundled ffmpeg, no external runtime). Chosen over extending `xcap`'s
    own still-work-in-progress `video_recorder()` (raw frames only, no
    encoder) — see DECISIONS.md for the full comparison. Only the
    primary display is supported, same scope limit TASK-009 set for
    screenshots
  - `media::MediaStorage` extended (not replaced): PNG methods are
    untouched; new `video_path`/`video_exists`/`delete_video` are their
    MP4 counterparts. Unlike PNGs there's no `save_video(bytes)` — the
    recording engine writes directly, incrementally, to the path
    `video_path` returns while the recording is in progress.
    `reconcile` (the startup orphan sweep) now also sweeps `.mp4` files,
    which as a side effect cleans up a video left behind by a recording
    that was never stopped (e.g. the app closed mid-recording) — no new
    code needed for that case specifically
  - `recording.rs` (new top-level module): `RecordingState`, a small
    Tauri-managed `Mutex<Option<InProgressRecording>>` bridging the
    two-phase `start`/`stop` commands — mirrors `active_process::
    ActiveProcessState`'s shape. Supports **at most one recording at a
    time, system-wide** (see DECISIONS.md). Its `start` method holds its
    lock across the *entire* validate-and-launch sequence, not just an
    initial check, so two concurrent start calls can't both launch a
    real native recording before either notices the other
  - `commands::recording` (new): `start_recording_capture` validates
    the request, generates the Capture id up front, and starts a real
    recording to `<id>.mp4`, returning only an in-progress marker (no
    Capture row exists yet). `stop_recording_capture` takes the
    in-progress handle, blocks until the video is finalized, and creates
    the Capture metadata row using that same pre-generated id
  - `CaptureService` gained `validate_recording_start` (shared
    validation, extracted from `create`/`create_screenshot` into a new
    private `validate_for_create` helper — behavior unchanged, now one
    shared implementation instead of two copies) and
    `finalize_recording(id, ...)` — unlike every other creation path,
    this one takes an externally-supplied id (the video was already
    written under it) rather than generating a fresh one, confirms the
    video file exists before inserting metadata, and cleans it up on a
    metadata-insert failure — the same discipline `create_screenshot`
    established for PNGs. `delete` now removes a capture's video
    alongside its PNG unconditionally (both are safe no-ops if absent)
  - `CreateCaptureDialog`'s Type = Recording two-phase flow
    **(superseded by TASK-014 below — moved to a toolbar-level
    control)**. Original TASK-013 shape kept here only for history: the
    dialog collected fields, then "Start recording" locked everything
    and became "Stop recording."
- **Recording UI and playback (TASK-014)** — makes screen recording
  actually usable end to end: a visible Start/Stop control with a live
  elapsed-time indicator, reachable from two places and correct
  regardless of which one started it, plus real video playback:
  - Start/Stop **moved out of `CreateCaptureDialog` entirely** into a
    toolbar-level `RecordingControl` in `CapturesSection` (sibling of
    "+ New capture") and a third button in the widget, alongside
    "Capture screenshot"/"Add marker". `CreateCaptureDialog`'s Type
    dropdown no longer offers "Recording" — a Recording Capture is only
    ever created through this dedicated flow now, the same rule
    Screenshot has followed since TASK-009 (see DECISIONS.md). Both
    controls use an auto-generated default title
    (`defaultRecordingTitle()`, `services/captures.ts`) — no dialog
  - Recording is **system-wide** (one at a time, unchanged from
    TASK-013), so both controls handle three states: nothing recording
    (Start enabled), recording *for this Process* (elapsed indicator +
    Stop), or recording *for a different Process* (Start disabled with
    an explanatory tooltip)
  - **Cross-window sync**: `recording::RecordingStatusInfo` (id,
    process, title, `started_at`) is broadcast via a new
    `recording-status-changed` event (`AppHandle::emit`) — the exact
    same `get_*_status`-on-mount + live-event-push shape TASK-011 built
    for the active Process. `RecordingState` gained a `status()`
    accessor (read-only snapshot, doesn't consume). `stop_recording_capture`
    emits the "cleared" event *immediately* after taking the recording
    out of state — before calling `handle.stop()`/`finalize_recording`,
    both of which can still fail — so no window is left showing a
    ticking indicator for a recording that isn't running anymore
  - `src/hooks/` (new top-level folder, first occupant
    `useElapsedSeconds`) + `src/utils/formatElapsed.ts`: shared
    second-by-second elapsed-time ticking/formatting (`M:SS`/`H:MM:SS`),
    used identically by the Captures section's toolbar and the widget
  - `CaptureDetail`: Recording captures now render a bounded
    `<video controls>` element (same non-overflowing media-box
    treatment as the screenshot `<img>`), fed by a new
    `getRecordingMediaUrl` (`services/captures.ts`) calling a new
    `get_recording_media` command (mirrors `get_capture_media` exactly
    — Capture id in, backend-derived MP4 bytes out as a raw IPC
    response, wrapped in a `blob:` URL). **Deliberately a single
    in-memory byte-buffer transfer**, not a streaming/range-request
    protocol (e.g. Tauri's `asset:` protocol) — evaluated and
    consciously scoped down rather than assumed necessary; seeking
    still works over a `blob:` URL since the whole file is already
    resident in memory once loaded (see DECISIONS.md for the full
    trade-off, including the memory/large-file caveat)
  - `CaptureService::get_recording_media` + `MediaStorage::read_video`
    (new): the Recording counterparts of `get_screenshot_media`/
    `read_capture`, same `AppError::NotFound`-for-no-media behavior
- **TASK-014 bugfixes** — found by manual use immediately after TASK-014
  was first reported done; fixed before starting TASK-015 (see
  DECISIONS.md, "TASK-014 bugfixes," for the full diagnosis story):
  - **Video playback didn't work at all.** Root cause:
    `native::recording::WindowsRecordingEngine` never set an explicit
    video codec, so `windows-capture` used its own default —
    `VideoSettingsSubType::HEVC` — which Chromium/WebView2 (what
    `CaptureDetail`'s `<video>` element runs on) has no built-in decoder
    for. Fixed by explicitly requesting `VideoSettingsSubType::H264`.
    TASK-013's native verification only ever confirmed the MP4
    *container* was well-formed, never which codec was inside it — a
    narrower check than it read as
  - **The widget wasn't draggable.** Root cause: `data-tauri-drag-region`
    (present on the header since TASK-011) needs
    `core:window:allow-start-dragging`, which was never actually granted
    — the widget's capability file only ever added
    `core:event:allow-listen` on top of `core:default`. Fixed by adding
    the permission
  - **The widget didn't look like a small dot.** Redesigned with two
    in-window states: a collapsed circular dot (56×56 requested,
    `.widget-dot`) that expands to the original panel on click, and a
    new "–" header button that collapses back (separate from "×", which
    still fully hides the OS window as before). The window is genuinely
    resized between states (`commands::widget::set_widget_expanded`),
    not just visually toggled with CSS. Windows enforces a minimum
    top-level window width (`SM_CXMIN`, ~130px) regardless of what's
    requested, so the "56×56" dot actually becomes a ~133×56 window;
    the dot is centered within whatever width the OS actually grants
    (`position: fixed` + a centering transform) rather than fighting
    that floor with native window-message interception
- **Microphone audio capture (TASK-015)** — closes the last capture-
  modality gap; **M1 — Live Capture is now complete**:
  - New dependency `cpal` (WASAPI-backed on Windows, no extra Cargo
    feature) — `windows-capture`'s own audio support is encode-only (it
    muxes PCM bytes you hand it, but doesn't open a microphone itself),
    so a second, genuinely separate crate captures the actual audio.
    Hand-rolling WASAPI directly was evaluated and rejected as real
    native protocol work `cpal` already solves — see DECISIONS.md
  - `native::recording::RecordingEngine::start` gained an
    `include_audio: bool` parameter. When true,
    `RecordingHandlerImpl::new` queries the microphone's own
    `default_input_config()` (sample rate/channel count — used as-is,
    no resampling), configures `AudioSettingsBuilder` to match, and
    opens a `cpal` input stream whose callback converts `f32`/`i16`
    samples to 16-bit PCM and feeds them into the same
    `VideoEncoder::send_audio_buffer` the video frames go through
  - The shared `VideoEncoder` is `Arc<Mutex<Option<VideoEncoder>>>` —
    video frames (`windows_capture`'s thread) and microphone samples
    (`cpal`'s thread) arrive on two genuinely different OS-driven
    threads; the `Option` lets `on_closed` `.take()` an owned value out
    for the consuming `VideoEncoder::finish()` call. The microphone
    stream is dropped *before* the encoder is finalized, so no audio
    callback can race `finish()`
  - `StartRecordingInput` gained `include_audio` (defaults to `false`).
    `CapturesSection`'s `RecordingControl` and the widget both gained an
    "Include microphone audio" checkbox — shown only before a recording
    starts (hidden once one is in progress), plain local state, not
    persisted or shared between the two surfaces (see DECISIONS.md)
  - No playback changes needed — `CaptureDetail`'s existing
    `<video controls>` element (TASK-014) plays embedded audio for free
    once the file actually has an audio track
  - Two new `#[ignore]`d native smoke tests
    (`record_primary_display_with_audio_smoke_test` mirrors the
    video-only one); a temporary spot-check confirmed a real recording
    made with audio enabled contains **both** `avc1` (H.264) and `mp4a`
    (AAC) sample-entry FourCCs — not just that the file is non-empty,
    applying the exact lesson TASK-013/014's codec bug taught (see
    DECISIONS.md)
- **Post-TASK-015 UI fixes, first pass** — five real problems found by
  manual use immediately after TASK-015. Only the toolbar-consolidation
  fix below actually held up under real testing; the other three were
  re-diagnosed and fixed for real in the second pass (see next bullet
  and DECISIONS.md, "Second UI bugfix pass," for the full corrected
  diagnosis):
  - **Toolbar hierarchy was unclear, and Start/Stop recording sat
    outside "new capture" options.** "+ New capture", the audio
    checkbox, and "Start recording" were consolidated into one
    `NewCaptureMenu` component — a single "+ New capture" button opening
    a popover with Screenshot / Note / Recording, the last holding the
    audio checkbox and Start action inside it. `RecordingControl` was
    deleted; `CreateCaptureDialog` gained an `initialType` prop so the
    popover's Screenshot/Note items open it pre-set. **This one worked.**
  - ~~Elements overflowed the window at narrower sizes — fixed by a
    defensive `overflow-x: auto`~~ — did not actually stop the overflow,
    see below.
  - ~~The widget dot had an ugly black rectangular background — fixed by
    `"backgroundColor"` config~~ — did not actually fix it, see below.
  - ~~A just-made capture didn't appear until leaving and returning —
    fixed by a `capture-created` event~~ — fixed the dialog-driven path
    only, missed the hotkey path, see below.
- **Post-TASK-015 UI fixes, second pass** — re-diagnosed from scratch
  with a real dev-server harness and a real native build/screenshot,
  after the user reported the three fixes above hadn't held up (full
  diagnosis in docs/architecture.md §30, decisions in DECISIONS.md
  "Second UI bugfix pass"):
  - **Overflow, real cause:** three header rows
    (`.workspace__header`/`.process-detail__header`/`.capture-detail__header`)
    are flex rows whose `min-width: auto` default refused to shrink
    below their Edit/Delete-actions-plus-title content width — no
    `overflow-x` rule anywhere stops a flex item from demanding more
    width than its container has in the first place. Confirmed as real,
    visible overflow (not hypothetical) by measuring
    `getBoundingClientRect()` against `window.innerWidth` at 760×480,
    the app's documented minimum window size. Fixed with `min-width: 0`
    on those three rows plus a new `.entity-header__titles` class on
    their title-block children, and — the change that actually mattered
    — `flex-wrap: wrap` on `.processes-layout`/`.captures-layout`/
    `.workspace-tabs`/`.workspace__actions` so a list+detail split drops
    to a stacked, full-width layout instead of squeezing both panes into
    unreadable slivers once the window is too narrow for both side by
    side. `.processes-list-pane` also lost its `flex-shrink: 0` (was a
    truly fixed 260px, unlike `.captures-list-pane`'s already-shrinkable
    treatment). Verified zero-overflow at 760×480 and a stricter 640×480
    stress test across every reachable screen in the app.
  - **Widget transparency, real cause:** the `"backgroundColor"` config
    field is real and correctly targets this problem per Tauri's docs,
    but per `WebviewWindow::set_background_color`'s own doc comment, a
    webview window's background on Windows is painted in three
    independent layers (native window / WebView2 control / page CSS),
    and the declarative config field is not guaranteed to reach the
    middle layer for a window declared via the `"windows"` array's
    implicit-default-webview shorthand (no separate `"webviews"` array)
    — which is how the widget window is declared. Fixed by *also*
    calling `WebviewWindow::set_background_color(Some(Color(0,0,0,0)))`
    explicitly at runtime in `lib.rs`'s `.setup()`. **Actually confirmed
    this time**: launched the freshly built `golive.exe`, minimized the
    main window via a P/Invoke `ShowWindow`/`SW_MINIMIZE` helper so the
    widget floated over bare desktop, and captured the screen region
    with `System.Drawing.Graphics.CopyFromScreen` — the image shows the
    desktop and an unrelated app panel cleanly through the transparent
    margin around the circle, no rectangular edge anywhere.
  - **Live sync, real cause:** the `capture-created` event and
    `CapturesSection` subscriber from the first pass were correct
    (re-verified by actually submitting `CreateCaptureDialog` in the
    harness and reading the list back) — but only creation paths that go
    through a `#[tauri::command]` function emit it. `hotkey.rs`'s
    `handle_capture_shortcut` (the global-hotkey screenshot, TASK-011)
    calls `CaptureService` directly and was never wired to this event,
    so a hotkey-triggered screenshot — the app's actual "capture without
    switching windows" feature — saved correctly but never told an open
    Captures section about it. Fixed by emitting
    `CAPTURE_CREATED_EVENT` from `handle_capture_shortcut` on success,
    reusing the same event/payload contract. Not independently
    re-verified by pressing the real global hotkey against a live main
    window (no way to drive the native window's UI in this environment)
    — verified by `cargo check`/`cargo test` (144 passed) passing
    against a real `Capture` value and the identical, already-proven
    emit call the dialog-driven path uses.
  - **Hierarchy:** one concrete, falsifiable fix — the Project Overview
    tab's "Captures" placeholder tile and the workspace's disabled
    "Captures" tab tooltip both told the user Captures didn't exist in
    the app at all, when it's been fully working since TASK-008/009,
    just nested inside a selected Process. Fixed: removed "Captures"
    from `ProjectOverview`'s `FUTURE_SECTIONS`, and gave the disabled
    tab a specific hint pointing at Processes instead of the generic
    "Not available yet". The broader "big hierarchy issues" complaint —
    the vaguest of the five original reports — is deliberately left open
    rather than guessed at further; pending the user's reaction to this
    build.
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

**TASK-010 validation:** `npx tsc --noEmit`, `npm run build`, `cargo
check` (no warnings), `cargo test` (119/119 passing — 117 pre-existing +
2 new `tray::tests`, covering `tray.rs`'s one piece of pure logic —
tooltip/menu-label text formatting — directly, since a real tray/menu
handle can't be constructed outside a running Tauri app), and `npm run
tauri build` all pass.

Full UI-flow verification (select a Process → tray sync call recorded
with `{ process_name, project_name }` → select a different Process →
re-synced → rename the active Process → re-synced with the new name →
delete the active Process → tray call is `null` → select the remaining
Process → delete its Project → tray call is `null` again → empty state
returns → Settings and every existing Project/Process/Capture flow
unaffected) was verified against the Vite dev server with the same
mocked-`window.__TAURI_INTERNALS__.invoke` approach as every prior task,
extended with a `set_active_process_tray` handler recording its calls
into `window.__mockTrayCalls`.

**Close-to-tray — manually verified against the compiled binary, not
simulated:** launched `golive.exe`, confirmed running via `tasklist`,
sent a graceful window-close request (`taskkill` without `/F`, which
delivers a real Windows close message rather than forcefully terminating
the process), and confirmed the *same process* (same PID) was still
running several seconds later — proof `WindowEvent::CloseRequested` was
intercepted and the window hidden rather than the app exiting. A
force-close (`taskkill /F`) afterward still worked, confirming the
documented escape hatch. What was **not** verified: actually clicking
the tray icon's "Open GoLive"/"Quit" menu items, or its "Active: …"
label rendering correctly — no desktop UI-automation tool is available
in this environment to interact with the native tray directly (the same
limitation TASK-003 through TASK-009 already recorded for the native
window itself). Given the graceful-close proof above and that the tray
build code compiled and ran without error (the tray icon must have been
successfully created for the app to have stayed alive/hidden rather than
crashed in `.setup()`), this is judged reasonable evidence the feature
works, but a developer with an interactive desktop session should still
click through the tray menu once before relying on it in a real
engagement.

**TASK-011 validation:** `npx tsc --noEmit`, `npm run build` (now
producing two HTML/JS/CSS bundles, `main` and `widget`, confirmed
separately sized in the build output — the widget's is a few KB, not the
whole app), `cargo check` (no warnings), `cargo test` (124/124 passing —
119 pre-existing + 5 new: `active_process::tests` covering the shared
state's start-empty/set-get/clear behavior, `hotkey::tests` pinning the
`CaptureResult` event's exact JSON tag shape and the shortcut
constructor's determinism), and `npm run tauri build` all pass.

Full UI-flow verification of the main window (select/create/rename/
delete a Process still calls the renamed `sync_active_process` command,
now with the fuller `{ process_id, process_name, project_id,
project_name }` shape; every existing Project/Process/Capture flow
unaffected) used the same mocked-`window.__TAURI_INTERNALS__.invoke`
dev-server approach as every prior task. The widget page (`widget.html`)
was additionally loaded directly in a browser tab: its "no active
process" empty state rendered correctly with the capture button
correctly disabled, and its hide button correctly called `hide_widget`.
The "active Process shown, capture succeeds" path specifically could not
be click-tested in this harness — the widget's on-mount fetch races the
test mock's injection with no retry affordance to recover from losing
that race (unlike the main app's "Retry" button) — but both of that
path's constituent calls are independently proven elsewhere (the main
window's regression pass; TASK-009's exhaustive screenshot-capture
verification), so this is judged a testing-harness gap, not an
unverified functional path.

**Native verification, not simulated — including a real bug this
process caught:**
- Launching the freshly built `golive.exe` initially **crashed on
  startup** with "Failed to setup app: ... HotKey already registered" —
  the originally-chosen shortcut (`Ctrl+Shift+G`) was already claimed by
  another application on the verification machine, and the code at the
  time propagated that failure as fatal. This was diagnosed and fixed
  (registration failure is now logged and non-fatal; the default
  shortcut was also changed) — see DECISIONS.md — and the fix was
  re-verified by rebuilding and relaunching successfully.
- After the fix, `golive.exe` was launched and, via a Win32 `EnumWindows`
  enumeration (PowerShell + P/Invoke — no desktop UI-automation tool can
  click the native UI directly, the standing limitation prior tasks
  already recorded), **both windows were confirmed to exist and be
  visible simultaneously**: `"GoLive"` (main) and `"GoLive Capture"`
  (widget).
- The global shortcut was triggered for real via low-level `keybd_event`
  Win32 input (genuine OS-level key-down/key-up events, not
  `SendKeys`, which only targets the foreground window and would not
  reliably trigger a true global hotkey) — the app remained running and
  responsive afterward (`Get-Process ... Responding` = `True`), and,
  since no active Process was set for that run, correctly created no
  capture file (confirmed via a direct `%APPDATA%\...\captures\`
  directory listing) — proof the `no_active_process` path executes
  cleanly without crashing.
- Close-to-tray was re-verified with both windows present, using the
  same graceful-close-request-survives-as-the-same-PID method TASK-010
  used, then force-closed for cleanup.
- What was **not** verified: an actual end-to-end "hotkey → real
  screenshot → appears in the active Process's list" pass, since setting
  a real active Process requires clicking through the native main
  window's UI (select a Project, a Process) — the same no-UI-automation
  limitation as ever. Given the capture pipeline itself was exhaustively
  verified in TASK-009 (real desktop content, real `%APPDATA%`
  persistence) and the hotkey handler calls that identical, unmodified
  function, this is judged low-risk, but a developer with an interactive
  desktop session should do one full unassisted pass (select a Process,
  press the hotkey, confirm the Capture appears) before relying on this
  in a real engagement.

**TASK-012 validation:** `npx tsc --noEmit`, `npm run build` (two
bundles as before, `main` and `widget`, the widget's grown only slightly
— 2.24 kB → still ~2.2 kB), `cargo check` (no warnings), `cargo test`
(124/124 passing — unchanged from TASK-011, since this task touched no
Rust code at all), and `npm run tauri build` all pass.

No frontend automated tests exist in this project (no test runner is
installed — see docs/architecture.md §2/§25); verification instead
directly exercised the real compiled module. Rather than repeat
TASK-011's known-losing race (the widget's on-mount fetch beats mock
injection every time, since there's no retry affordance to recover
from losing that race — documented there, still true here for the same
reason), this task's UI verification dynamically `import()`ed the
compiled `features/projects/services/captures.ts` module directly in a
browser tab already loaded against the Vite dev server, with
`window.__TAURI_INTERNALS__.invoke` mocked *before* the import (so there
was no mount-order race to lose — the module was loaded fresh into an
already-mocked environment). Calling `createQuickMarker("proc-123")`
against the mock confirmed it invokes `create_capture` with exactly
`{ input: { process_id: "proc-123", capture_type: "note", title:
"Marker — Aug 18, 2026, 10:33 PM", description: "" } }` and correctly
maps the mocked response back into a `Capture` — i.e. the new code's
actual wire behavior was verified directly, not just read. Separately,
`widget.html` was loaded directly and its "no active process" empty
state was confirmed to render *both* buttons (`Capture screenshot`,
`Add marker`), both correctly `element.disabled === true`.

The "active Process set → click Add marker → visible Capture" path
could not be click-tested end-to-end in this harness, for the identical
reason TASK-011 already documented for its screenshot button — not a
new gap. Given `create_capture`/`CaptureService::create` themselves are
exhaustively covered (TASK-008's 16 service tests plus that task's own
full mocked-IPC UI pass) and this task added no new backend surface for
them to call into, this is judged sufficient.

**Native verification, not simulated:** the freshly rebuilt `golive.exe`
was launched and confirmed to start cleanly (no crash) via `tasklist`;
the startup media-reconciliation sweep ran and removed orphaned files
left over from earlier verification sessions, exactly as designed. A
Win32 `EnumWindows` enumeration (PowerShell + P/Invoke, the same
standing technique TASK-010/011 used) confirmed both windows —
`"GoLive"` and `"GoLive Capture"` — render correctly at the widget's new
210px height. What was **not** verified: an actual end-to-end "select a
Process → click Add marker → see it in the Captures list" pass through
the native window, for the same no-desktop-UI-automation limitation
every prior task has recorded. A developer with an interactive desktop
session should do one full unassisted pass (select a Process, open the
widget, click "Add marker," confirm a Note Capture titled `Marker —
<timestamp>` appears in that Process's Captures list) before relying on
this in a real engagement.

**TASK-013 validation:** `npx tsc --noEmit`, `npm run build`, `cargo
check` (no warnings), `cargo test` (139/139 passing — 124 pre-existing +
15 new: 5 `media::tests` for `video_path`/`video_exists`/`delete_video`/
widened `reconcile`, 5 `services::capture::tests` for
`validate_recording_start`/`finalize_recording`/`delete` now covering
video, and 5 `recording::tests` against a fake `RecordingHandle` for
`RecordingState`'s start/reject-concurrent/take semantics), and `npm run
tauri build` all pass — including a genuinely new dependency,
`windows-capture`, compiling cleanly against its real API on the first
attempt.

Full UI-flow verification (Projects → Process → Captures → "+ New
capture" → Type = Recording → title filled → "Start recording" clicked
→ confirmed via the mocked backend that `start_recording_capture` was
called with the exact expected `{ process_id, title }` shape, every
field and Cancel became `disabled`, and the button relabeled to "Stop
recording" → clicked → confirmed `stop_recording_capture` was called,
the dialog closed, and the finished Recording Capture appeared
auto-selected in the list with a "Recording" badge, exactly like every
other capture-creation flow) was verified against the Vite dev server
with the same mocked-`window.__TAURI_INTERNALS__.invoke` approach every
prior task has used, extended with `start_recording_capture`/
`stop_recording_capture` handlers (the mock also rejects a second
concurrent start, mirroring the real backend's behavior).

**Native verification, not simulated — the real risk this task existed
to de-risk:** `cargo test --release -- --ignored
record_primary_display_smoke_test` was run explicitly on this session's
real Windows desktop and **passed** — a genuine ~2-second recording of
the primary display produced a non-empty, valid MP4 via the real
Windows Graphics Capture API (not a mock, not a stub). A further
one-off spot-check (`examples/recording_spotcheck.rs`, written
temporarily and deleted before this task was considered complete, with
`lib.rs`'s `native`/`errors` modules briefly widened to `pub` to reach
them directly — same temporary-widening convention TASK-004 through
TASK-009 established, reverted immediately after) recorded ~4 real
seconds of this session's actual desktop to a non-temp-dir path for
direct inspection: the output was **156,431 bytes**, and its first
bytes were verified byte-for-byte to be a well-formed ISO-BMFF/MP4 box
header (a 24-byte `ftyp` box with major brand `mp42`) — decisive,
concrete evidence of a genuine, correctly-encoded video file, not empty
output or garbage bytes. `git status` was checked afterward to confirm
no stray files (the temporary example, the temporary `pub` widening, or
the spot-check's output directory) remained.

The freshly rebuilt `golive.exe` was also launched and, via the same
Win32 `EnumWindows` enumeration prior tasks established, both windows
were confirmed to render with no crash. What was **not** verified: an
actual end-to-end "click Start recording in the native UI → wait →
click Stop → see the Capture in the list" pass through the compiled
app's own window — the same no-desktop-UI-automation limitation every
prior task has recorded. Between the mocked-IPC pass (proving the
*frontend* wiring) and the native smoke test/spot-check above (proving
the *native engine* independently, against real desktop content), this
is judged sufficient without overclaiming a native-UI click-through
that couldn't actually be performed here. A developer with an
interactive desktop session should still do one full unassisted pass
through `golive.exe` (start a recording, stop it, confirm the video
file at `%APPDATA%\com.golive.app\captures\<id>.mp4` plays correctly in
a real media player such as Windows' own Movies & TV app) before
relying on this in a real engagement.

**TASK-014 validation:** `npx tsc --noEmit`, `npm run build`, `cargo
check` (no warnings), `cargo test` (144/144 passing — 139 pre-existing +
5 new: `recording::tests::status_reflects_the_current_in_progress_recording`,
2 `media::tests` for `read_video`, 2 `services::capture::tests` for
`get_recording_media`), and `npm run tauri build` all pass.

Full UI-flow verification (Projects → Process → Captures → toolbar's
"Start recording" → confirmed `start_recording_capture` called with the
exact expected `{ process_id, title }` shape → the toolbar switched to a
live "● Recording M:SS" indicator, confirmed to actually tick across a
real multi-second wait, not just render once → "Stop recording" →
confirmed `stop_recording_capture` was called, the toolbar returned to
"Start recording," and the finished Recording Capture appeared
auto-selected with a "Recording" badge → its `CaptureDetail` was
confirmed to render a real `<video>` element with a `blob:` `src` built
from the mocked `get_recording_media` response, no loading/error state
left showing) was verified against the Vite dev server with the same
mocked-IPC approach every prior task has used, extended with a
`transformCallback` stub so `listen()` doesn't throw in this harness.
The widget's three-button layout (all correctly `disabled` with no
active Process) was confirmed the same way TASK-011/012 verified their
own buttons; the recording-specific service functions the widget
depends on (`defaultRecordingTitle`, `startRecordingCapture`,
`getRecordingStatus`) were additionally verified directly via dynamic
`import()` against a pre-mocked environment (the same technique
TASK-012 used for `createQuickMarker`), confirming their exact wire
shapes. Genuine cross-window event *delivery* (a recording started in
one window showing up live in another) still can't be exercised in this
harness — the same standing limitation TASK-011 already documented for
`active-process-changed`, now also covering `recording-status-changed`;
not a new gap, and independently proven correct by the Rust-side
`recording::tests` plus the `commands::recording` emit code itself.

**Native verification:** the freshly rebuilt `golive.exe` was launched
and, via the same Win32 `EnumWindows` check every prior task has used,
both windows were confirmed to render correctly with no crash. **This
paragraph originally claimed "no further native smoke test was needed"
here, reasoning TASK-013's real-recording verification already retired
the native risk — that turned out to be wrong** (see the bugfix
validation below): a well-formed MP4 container and a *playable* one are
different claims, and only the first had actually been checked. What
was **not** verified even after the bugfixes below: an actual "start a
recording from the widget while the main window is hidden, stop it,
open the main window, watch it play back" pass through the compiled
app's own window — the roadmap step's own definition of done — for the
same no-desktop-UI-automation limitation every prior task has recorded.
Given the first pass already got this wrong once, this is flagged more
strongly than usual: a developer with an interactive desktop session
should do that exact pass before relying on this in a real engagement.

**TASK-014 bugfix validation (video playback + widget dot/drag):**
`npx tsc --noEmit`, `npm run build`, `cargo check` (no warnings),
`cargo test` (144/144 — unchanged, none of these fixes needed new
automated tests: the codec choice and window-resize logic both require
a real native window/capture session, matching this project's existing
"can't be constructed outside a running app" precedent), and
`npm run tauri build` all pass.

**Video codec fix — verified with a real recording, not by re-reading
docs:** a temporary spot-check (`examples/recording_codec_spotcheck.rs`,
deleted before this fix was complete, same temporary-`pub`-widening
convention as prior native spot-checks) recorded 3 real seconds of this
session's desktop and scanned the raw MP4 bytes for the codec's actual
sample-entry FourCC: `avc1` (H.264) was present, `hvc1`/`hev1` (HEVC)
were absent — decisive, not inferred.

**Dot centering — verified geometrically, not visually** (no
native-window screenshot tool exists in this environment, the same
limitation every prior task has recorded): the Vite dev server's
`widget.html` was loaded in a browser tab resized to 134×56 — a
stand-in for the real OS-floored window size, confirmed against the
actual compiled app via `EnumWindows`/`GetWindowRect`
(`left=102 top=102 width=133 height=56`, matching the predicted
`SM_CXMIN` floor) — and the dot's `getBoundingClientRect()` was checked:
`x≈38.8` against a 56px-wide dot in a 134px-wide viewport,
`(134-56)/2=39` — centered to within rounding.

**Click-to-expand/collapse — verified interactively:** with a mocked
`window.__TAURI_INTERNALS__.invoke`, clicking the dot called
`set_widget_expanded(true)` and the expanded panel rendered; clicking
its new "–" button called `set_widget_expanded(false)` and the dot
rendered again; the "×" button still called `hide_widget` afterward,
confirming it wasn't accidentally repurposed.

**Native verification:** the freshly rebuilt `golive.exe` was launched
and, via the same Win32 `EnumWindows`/`GetWindowRect` check every prior
task has used, the widget window was confirmed to actually be ~133×56
at rest (not the old 260×250 panel size) with no crash. What was
**not** verified: an actual native click on the dot, an actual
click-and-drag with the mouse, or genuine HEVC-vs-H.264 playback inside
the compiled app's own `<video>` element (only the underlying MP4 bytes
were inspected directly) — still the same no-desktop-UI-automation
limitation. A developer with an interactive desktop session should
confirm all three by hand (drag the dot around the screen, click it,
and watch a real recording play back) before relying on this in a real
engagement.

**TASK-015 validation:** `npx tsc --noEmit`, `npm run build`, `cargo
check` (no warnings), `cargo test` (144/144 — unchanged; the new audio
code has no automated coverage beyond an `#[ignore]`d smoke test, since
real microphone capture can't be exercised deterministically headlessly,
matching the existing screen-capture precedent), and `npm run tauri
build` all pass.

Full UI-flow verification (Captures section → checked "Include
microphone audio" → clicked "Start recording" → confirmed via the
mocked backend that `start_recording_capture` was called with
`include_audio: true` exactly → confirmed the checkbox disappeared once
the recording was in progress, replaced by the elapsed indicator +
Stop) was verified against the Vite dev server with the same
mocked-IPC approach every prior task has used.

**Native verification, not simulated — applying TASK-013/014's own
lesson that "a file exists" isn't proof of correctness:**
`cargo test --release -- --ignored
record_primary_display_with_audio_smoke_test` was run on this session's
real desktop with a real microphone and **passed**. A further temporary
spot-check (deleted before this task was complete, same temporary-`pub`-
widening convention as every prior native spot-check) recorded 5 real
seconds with audio enabled and scanned the raw MP4 bytes for sample-
entry FourCCs: **both** `avc1` (H.264 video) and `mp4a` (AAC audio)
were present, `hvc1` (HEVC) was not — concrete, byte-level proof both
tracks were genuinely muxed into one file. `git status` was checked
afterward to confirm no stray files remained. The freshly rebuilt
`golive.exe` was also launched and, via the same Win32 `EnumWindows`
check every prior task has used, both windows were confirmed to render
correctly with no crash.

What was **not** verified: an actual native-UI pass (check the box in
the compiled app, click Start recording, speak, click Stop, and confirm
the played-back video has synchronized, audible speech) — the roadmap
step's own definition of done, and the same no-desktop-UI-automation
limitation every prior task has recorded. The byte-level check above
proves an audio track exists; it does not independently prove the audio
stays synchronized with the video over a longer recording (relies on
`windows-capture`'s own internal audio-clock handling — see "Known
technical risks" below). A developer with an interactive desktop
session and a working microphone should do that exact pass — real
speech against real screen content, per the roadmap's own definition of
done — before relying on this in a real engagement.

**Post-TASK-015 UI fixes validation, first pass (superseded — see below):**
`npx tsc --noEmit`, `npm run build`, `cargo check` (no warnings),
`cargo test` (144/144 — unchanged; these are UI/config changes with no
new pure logic to test), and `npm run tauri build` all pass.

The overflow fix was verified two ways: by removing the actual CSS
trigger (the old three-control toolbar), and by checking
`document.documentElement.scrollWidth` against `clientWidth` at a
deliberately narrow 780×520 viewport (near the main window's enforced
760px minimum) in the mocked dev-server harness — `false` for
`hasHorizontalOverflow`, both with the new "+ New capture" popover
closed and open. **This measurement was real but insufficient** — it
checked the document's own scroll width, which stayed flat because a
distant ancestor's `overflow-x: auto` was silently absorbing the
overflow into a scrollbar rather than the overflow not existing; it
didn't check whether any individual element's own bounding box exceeded
the viewport, which is what "elements overflow the window" actually
meant and what the second pass below found still failing. The
`capture-created` live-sync fix was verified by directly invoking a
simulated event callback with a capture payload this window never
created itself, confirming it appeared in the list — correct as far as
it went, but it only exercised the command-layer emit path, not the
global-hotkey path that turned out to be the actual gap.

The widget dot's transparency fix could **not** be visually confirmed —
no native-window screenshot capability was used in this session. It
turned out this caveat was hiding a real bug, not just an unconfirmed-
but-correct fix — see below.

**Post-TASK-015 UI fixes validation, second pass:** `cargo check`,
`cargo test` (144 passed, 3 ignored, unchanged), `tsc --noEmit` all
clean. This time each fix was verified against the actual failure mode,
not just re-reasoned about:

- **Overflow:** scripted `getBoundingClientRect()` sweeps checking every
  element's own edges against `window.innerWidth` (not just the
  document's scrollWidth) at 760×480 (the app's real minimum) and
  640×480 (a stricter stress test) — zero overflow after the fix,
  real/measured overflow before it (e.g. an actions block at
  `right: 817` against a 760px viewport). Exercised across every
  reachable screen: Projects list, a Project's Overview and Processes
  tabs, a selected Process's Captures list and a selected Capture's
  detail pane, and the New Project dialog — not just the one page the
  original bug report screenshotted.
- **Widget transparency:** genuinely visually confirmed this time.
  Launched the freshly built `golive.exe` directly (after killing a
  stale running instance), minimized the main window via a small
  `ShowWindow`/`SW_MINIMIZE` P/Invoke helper so the widget floated over
  bare desktop, and captured the screen region with
  `System.Drawing.Graphics.CopyFromScreen`, then read the resulting PNG
  back as an image. It shows the desktop wallpaper and an unrelated app
  panel cleanly through the widget window's transparent margin around
  the 56px circle, with no rectangular edge or seam anywhere — proof,
  not documentation-based confidence. Drag behavior in either widget
  state is still not verified (still no way to simulate real
  click-and-drag against a native window in this environment).
- **Live sync (hotkey path):** the dialog-driven path was re-verified
  by actually submitting `CreateCaptureDialog` in the mocked harness and
  reading the resulting list back (not just re-reading the code). The
  hotkey path's fix (`hotkey.rs` now also emits `CAPTURE_CREATED_EVENT`)
  is **not** independently verified end to end — pressing the real
  global hotkey and watching a live main window update — since this
  environment has no way to drive the native window's UI to confirm it
  visually. Confidence here rests on the fix reusing the exact,
  already-proven `app.emit(CAPTURE_CREATED_EVENT, &capture)` call and
  payload contract against a real `Capture` value, plus a clean
  `cargo check`/`cargo test`. **A developer should press the actual
  global hotkey (Ctrl+Alt+Shift+S) with a Process open in the main
  window and confirm the screenshot appears in its Captures list without
  leaving the page, before fully trusting this one.**
- **Hierarchy:** the one concrete fix (Overview tile + tab tooltip) was
  confirmed live in the mocked harness by reading back the rendered
  text/tooltip after the change. The broader hierarchy complaint is
  explicitly not claimed as resolved — see "Not implemented yet" /
  pending user feedback.

## Not implemented yet

- Note capture media (Note Captures other than quick markers remain
  metadata only — never had media to begin with, unlike Screenshot/
  Recording)
- Recording hotkey (TASK-014 added Start/Stop buttons to the Captures
  section and the widget, but no global keyboard shortcut for
  recording — only the screenshot hotkey exists, TASK-011)
- Editing/trimming video, thumbnails (never in scope for M1)
- Monitor selection, area/window-selection screenshots and recordings
  (TASK-009/TASK-013 only support "capture the primary/current display")
- Noise suppression/audio processing (TASK-015 records the microphone's
  raw signal as-is — explicitly out of scope, see roadmap.md)
- Standalone audio-only captures ("voice memo" — not part of the
  `CaptureType` set, explicitly out of scope, see roadmap.md TASK-015)
- Recording pause/resume
- Cancel/discard an in-progress recording (once started, the only way
  out is Stop — see DECISIONS.md TASK-013)
- A remembered "include microphone audio" preference (the checkbox
  resets every time — see DECISIONS.md TASK-015 for why this was a
  deliberate choice, not an oversight)
- A more specific error message when audio recording fails specifically
  because no microphone is available (currently surfaces the same
  generic "Failed to start screen recording." any other startup failure
  does — see docs/architecture.md §28)
- Hotkey customization UI (the combination is a hardcoded constant,
  `hotkey::shortcut()`)
- AI integration (OpenAI), structured process generation
- Transcription
- Process editor, process versioning
- Word export
- ZIP import/export
- Search (FTS5)
- Windows Credential Manager integration

## Known technical risks

- Native screen capture/recording across multi-monitor setups (TASK-009/
  TASK-013 deliberately support only the primary/current display;
  monitor selection is unstarted)
- Recording file size/performance over longer durations than this
  task's short manual verification exercised (a few seconds) — not yet
  measured for a realistic multi-minute consulting session
- Recording video playback for very long/large files: TASK-014
  deliberately transfers the whole MP4 as one in-memory byte buffer
  (same approach as screenshots) rather than a streaming/range-request
  protocol — untested at realistic multi-minute-recording file sizes;
  revisit if this becomes a real problem (see DECISIONS.md)
- Audio/video synchronization over longer recordings: TASK-015's native
  verification confirmed an audio track is genuinely muxed into the
  file (real recording, byte-level codec check), but relies on
  `windows-capture`'s own "monotonic audio clock" for actual sync —
  not independently verified against real, extended speech content; a
  developer with a working microphone should confirm this by hand (see
  docs/architecture.md §28) before relying on it in a real engagement
- ~~Widget dot transparency: unverified~~ — resolved. The second UI
  bugfix pass genuinely screenshotted the running widget (see
  docs/architecture.md §30) and confirmed a real transparent circle,
  not a rectangle. The one piece still unverified: dragging the widget
  in either state — still no way to simulate real click-and-drag against
  a native window in this environment
- Global-hotkey capture-created sync: `hotkey.rs` now emits
  `CAPTURE_CREATED_EVENT` on a successful hotkey screenshot (second UI
  bugfix pass), reusing the exact call/payload the already-proven
  dialog-driven path uses, and `cargo check`/`cargo test` pass against
  it — but this was **not** independently verified end to end (pressing
  the real global hotkey with a live main window open and watching its
  Captures list update), since this environment can't drive the native
  window's UI. A developer should do that one manual check before fully
  trusting it
- UX hierarchy feedback: one concrete, misleading-placeholder bug was
  found and fixed (see docs/architecture.md §30), but "the UX has some
  big hierarchy issues" — the vaguest of the five original reports — is
  not claimed as resolved. Two of the first pass's three other fixes
  also turned out not to hold up under real testing before this second
  pass caught it; treat any further UI-fix claim from this project as
  wanting the same real-harness/real-screenshot verification standard
  the second pass used, not just code review, before it's trusted
- AI integration reliability (structured/schema-constrained output)
- Word document generation quality for a consulting-grade deliverable

## Next task

TASK-016 (see roadmap.md) — the first step of M2 (AI Structuring):
Windows Credential Manager integration and AI settings. Blocked on the
user confirming the second UI bugfix pass (this session) actually holds
up — in particular the global-hotkey live-sync fix and the broader
hierarchy complaint, neither fully verifiable from this environment —
before treating M1's UI as genuinely settled.
