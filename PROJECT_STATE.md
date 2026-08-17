# PROJECT_STATE

Project:
GoLive

Current milestone:
M0 — Foundation

Completed:
TASK-001 (partially — see Blocked below)

## Current implementation

- Tauri 2 desktop application shell
- React + TypeScript frontend (Vite)
- Rust backend with a single proof-of-life command
  (`check_foundation_status`) verifying React → Tauri → Rust connectivity
- Windows desktop application window (title "GoLive")
- Basic application shell displaying "GoLive" / "Project foundation ready."
  and a live backend connectivity status
- Frontend folder scaffold (`components/`, `features/`, `pages/`,
  `services/`, `stores/`, `types/`, `utils/`) with placeholder READMEs
  describing intended purpose — no logic in any of them yet
- `docs/architecture.md` documenting the current layering and folder
  structure

## Blocked — environment prerequisite missing

The Rust backend could **not** be verified to compile or launch on this
development machine. `cargo check` fails while linking with:

```
LINK : fatal error LNK1181: cannot open input file 'kernel32.lib'
```

Diagnosis: Visual Studio 2022 Community is installed with the C++ Build
Tools workload (MSVC compiler and linker are present and were located at
`C:\Program Files\Microsoft Visual Studio\2022\Community\VC\Tools\MSVC\...`),
but the **Windows 10/11 SDK component is not installed** — there is no
`kernel32.lib` anywhere on disk, and `vswhere -requires
Microsoft.VisualStudio.Component.Windows10SDK` returns nothing. The MSVC
linker cannot produce any Windows binary without the SDK's import
libraries, so this affects every Rust crate, not just Tauri.

This is a machine setup issue, not a code issue — no source change can work
around it. It was reported to the user; auto-installing the SDK (a system
change requiring a sizeable download via the Visual Studio Installer) was
deliberately not performed without explicit confirmation.

**Fix:** open Visual Studio Installer → Modify → enable "Windows 10 SDK" or
"Windows 11 SDK" under the Desktop development with C++ workload (or add
the `Microsoft.VisualStudio.Component.Windows10SDK` /
`...Windows11SDK.2xxxx` component), then re-run:

```bash
cd src-tauri
cargo check
```

Once that succeeds, also verify `npm run tauri dev` (window launches,
status line reads "Rust backend connected.") and `npm run tauri build`
(produces an NSIS installer under
`src-tauri/target/release/bundle/nsis/`).

**Verified independently of this blocker (pure frontend, no Rust
involved):**
- `npm install` — succeeds
- `npx tsc --noEmit` — no errors
- `npm run build` (Vite production frontend build) — succeeds

**Not yet verified (blocked on the above):**
- `cargo check` / Rust compilation
- `npm run tauri dev` (development launch)
- `npm run tauri build` (production build, incl. NSIS installer)
- Actual application window launching on Windows

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
- Windows installer packaging validation (bundle target is configured but
  an installer has not yet been produced/tested end-to-end)

## Known technical risks

- Windows screen recording implementation (highest risk — flagged for
  incremental proof-of-concept development)
- Microphone capture and audio/video synchronization
- Native screen capture across multi-monitor setups
- AI integration reliability (structured/schema-constrained output)
- Word document generation quality for a consulting-grade deliverable

## Next task

Resolve the Windows SDK blocker above, re-run `cargo check` / `npm run
tauri dev` / `npm run tauri build` to confirm the foundation actually
launches, then proceed to TASK-002.
