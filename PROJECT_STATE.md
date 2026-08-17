# PROJECT_STATE

Project:
GoLive

Current milestone:
M0 — Foundation

Completed:
TASK-001, TASK-002, TASK-003

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
- Projects placeholder page — empty state, no project data or persistence
- Settings placeholder page — empty state, no settings implemented
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

## Not implemented yet

- Database (SQLite), migrations, repositories
- Projects (create/list/edit)
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

TASK-004
