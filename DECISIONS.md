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
