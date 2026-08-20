# GoLive

GoLive is a Windows desktop application that helps consultants capture user
processes and requirements **while users are performing their actual work**,
instead of reconstructing them afterwards from interviews and memory. It
records screenshots, screen video, microphone audio, notes, and markers
during a live working session, then uses cloud AI to turn that raw capture
into a structured, editable business process — and finally into a Word
functional specification.

## Current status

**MVP complete (TASK-001 through TASK-021), plus a Post-MVP round
(TASK-022).** GoLive implements the full workflow its own product
description promises: create a **project**, document a **process**
within it, capture that process live — from anywhere in Windows,
without switching to GoLive's main window — via a global hotkey and a
small always-on-top floating widget, in every modality the product
promises (screenshots, screen recordings with optional microphone
audio, and one-click markers, plus ordinary typed notes from the main
window), then turn those captures into a structured, AI-generated
business process using OpenAI, edit that draft by hand, and export the
result — as a Word (`.docx`) functional specification with embedded
screenshots, or as a LaTeX source bundle, the user's choice — via a
native Save As dialog. The Word export opens correctly in Microsoft
Word; the LaTeX bundle is a `document.tex` plus its embedded images and
a short README explaining how to compile it.

Everything persists locally in SQLite (and captured media alongside it
on disk); deleting a project cascades to its processes, captures, and
generated drafts. The OpenAI API key lives in the Windows Credential
Manager only — never in GoLive's own database or any file it writes.
GoLive keeps running in the system tray when its main window is closed;
the tray is the one real way to quit, and only one instance ever runs
at a time.

See [PROJECT_STATE.md](PROJECT_STATE.md) for the authoritative current
state (including known limitations and technical risk), and
[roadmap.md](roadmap.md) for how the project got here. Explicitly
deferred beyond this MVP: full-text search, ZIP project import/export,
monitor/area selection for capture, hotkey customization, recording
pause/resume, and any AI provider other than OpenAI — see roadmap.md's
"Explicitly deferred" section for the complete list and why.

## Development prerequisites

- **Windows 10/11**
- [Node.js](https://nodejs.org/) 18+ and npm
- [Rust](https://www.rust-lang.org/tools/install) (stable toolchain, via `rustup`)
- Tauri 2 system prerequisites (Microsoft Visual Studio C++ Build Tools and
  WebView2 — see the [Tauri prerequisites guide](https://tauri.app/start/prerequisites/)
  for Windows)

## Install dependencies

```bash
npm install
```

## Run in development

```bash
npm run tauri dev
```

This starts the Vite dev server and launches the GoLive desktop window with
hot reload.

## Build for production

```bash
npm run tauri build
```

Produces an optimized frontend bundle, a compiled Rust binary, and a Windows
installer under `src-tauri/target/release/bundle/`.

## Project structure

```
src/            React + TypeScript frontend
  components/   reusable presentation-only UI pieces
  features/     feature-scoped modules (projects, captures, processes, ...)
  pages/        top-level routed views
  services/     wrappers around Tauri invoke() calls
  stores/       cross-feature client-side state
  types/        shared TypeScript domain types
  utils/        small pure helpers
src-tauri/      Rust backend and Tauri configuration
  migrations/   versioned SQLite schema migrations
docs/           architecture and design documentation
tests/          automated test suite
```

Rust-side tests (database, repositories) run with `cargo test` from
`src-tauri/`; see [docs/architecture.md](docs/architecture.md) §17 for the
database architecture.

See [docs/architecture.md](docs/architecture.md) for the layering principles
and [DECISIONS.md](DECISIONS.md) for why key technologies were chosen.
