# components

Small, reusable, presentation-focused UI building blocks with no
feature-specific business logic (buttons, inputs, cards, layout primitives).

- `layout/` — application shell scaffolding (`AppShell`, `Sidebar`,
  `Header`). Not feature-specific; used exactly once each, by `App.tsx`.
- `ui/` — small generic pieces reused across more than one page
  (currently `EmptyState`, used by both `ProjectsPage` and
  `SettingsPage`).

Only add a component here once it's actually reused or is clearly
structural (like the layout pieces above) — one-off page content stays in
its page.
