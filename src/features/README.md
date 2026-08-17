# features

Feature-scoped modules (e.g. projects, captures, processes, settings), each
owning the code that only that feature needs: components, services
(Tauri calls), hooks, types, local state.

**Start simple, split only when complexity justifies it.** A new feature
starts as a single file or a small flat folder. Only break it into
`components/`, `services/`, `hooks/`, `types/`, `tests/` subfolders once it
actually has enough of each to need the separation — don't scaffold empty
subfolders in advance.

## Current contents

- `projects/` — the Project domain (list, create, delete, a Project
  Workspace with Overview/Processes tabs — Captures/Documentation still
  reserved) **and**, nested under it, the Process domain (a Process
  belongs to exactly one Project, so its UI lives inside the Project
  Workspace rather than as a sibling top-level feature):
  - `ProjectsView.tsx` — feature root; owns the project list and
    `activeProject` (list vs. workspace) state.
  - `components/` —
    `ProjectList`, `ProjectWorkspace` (workspace shell: back nav, header,
    tabs), `ProjectOverview`, `CreateProjectDialog`, `EditProjectDialog`,
    `DeleteProjectDialog` for the Project domain; `ProcessesView` (owns
    the process list + selection, rendered by `ProjectWorkspace`'s
    "Processes" tab), `ProcessList`, `ProcessDetail`,
    `ProcessStatusBadge`, `CreateProcessDialog`, `EditProcessDialog`,
    `DeleteProcessDialog` for the Process domain.
  - `services/projects.ts` / `services/processes.ts` — Tauri `invoke()`
    wrappers, per the "feature-specific service calls live in
    `features/<feature>/services/`" convention from TASK-002. Process
    types (`Process`, `ProcessStatus`, `*Input`) live in
    `services/processes.ts`, same pattern as `projects.ts`.
  - No `hooks/`, `types/` (domain types live directly in each service
    file), or `tests/` subfolder yet — nothing needs them.
