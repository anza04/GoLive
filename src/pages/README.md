# pages

Top-level views that compose components into full screens.

- `ProjectsPage.tsx` — placeholder empty state; no project data exists yet.
- `SettingsPage.tsx` — placeholder empty state; no settings exist yet.

Pages compose `components/` and stay free of business logic and Tauri
calls. `App.tsx` currently selects which page to render via local
navigation state — see docs/architecture.md ("Navigation") for how this
maps onto real routing later, and `PROJECT_STATE.md`/`TASK_INDEX.md` for
what functionality these pages still need.
