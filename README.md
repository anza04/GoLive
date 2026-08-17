# GoLive

GoLive is a Windows desktop application that helps consultants capture user
processes and requirements **while users are performing their actual work**,
instead of reconstructing them afterwards from interviews and memory. It
records screenshots, screen video, microphone audio, notes, and markers
during a live working session, then uses cloud AI to turn that raw capture
into a structured, editable business process — and finally into a Word
functional specification.

## Current status

**Milestone M0 — Foundation.** GoLive can now create, list, select, and
delete **projects** — the first real product functionality, persisted
locally in SQLite and surviving application restarts. The application
shell has a sidebar (Projects / Settings), a header with a live backend
connectivity indicator, and a Settings page showing local storage status.
No process/capture/recording/AI functionality exists yet — a project is
currently just a name, description, and creation/update dates, with a
detail view that clearly marks Processes/Captures/Documentation as "Not
available yet." See [PROJECT_STATE.md](PROJECT_STATE.md) for the
authoritative current state and [TASK_INDEX.md](TASK_INDEX.md) for what's
planned next.

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
