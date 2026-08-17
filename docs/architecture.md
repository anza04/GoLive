# Architecture

## Stack

- **Desktop shell:** Tauri 2
- **Frontend:** React + TypeScript (Vite)
- **Backend/native layer:** Rust
- **Database:** SQLite (not yet introduced — see TASK_INDEX.md)
- **Target platform:** Windows desktop only (MVP)

## Layering

```
React (frontend)
  ↓ invoke()
Tauri command (src-tauri/src, thin)
  ↓
Rust service          (introduced when business logic exists)
  ↓
Repository            (introduced with SQLite)
  ↓
SQLite
```

React never accesses the filesystem, database, or native OS APIs directly.
All such access goes through a Tauri command. This keeps business logic
testable independently of the UI and keeps a future cloud/server backend
possible without rewriting the frontend.

## Frontend structure

```
src/
  components/   reusable, presentation-only UI pieces
  features/     feature-scoped modules (projects, captures, processes, ...)
  pages/        top-level routed views composing features
  services/     wrappers around Tauri invoke() calls
  stores/       cross-feature client-side state
  types/        shared TypeScript domain types
  utils/        small pure helpers
```

Folders exist as placeholders (each with a short README) ahead of the
features that will populate them; see PROJECT_STATE.md for what is actually
implemented today.

## Backend structure

`src-tauri/src` currently contains only `main.rs` and `lib.rs` with a single
proof-of-life command. Future tasks will introduce `commands/`, `services/`,
`repositories/`, and `models/` modules as real functionality is added —
these are intentionally not scaffolded yet to avoid speculative structure.

## Status

This document reflects the foundation established in TASK-001. It will be
expanded as further architectural decisions are made (see DECISIONS.md).
