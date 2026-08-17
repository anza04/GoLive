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

- `projects/` — the Project domain (list, create, select, delete,
  detail view). The first feature to actually need the
  `components/`/`services/` split described above:
  - `ProjectsView.tsx` — feature root; owns list/selection/dialog state.
  - `components/` — `ProjectList`, `ProjectDetail`, `CreateProjectDialog`,
    `DeleteProjectDialog`.
  - `services/projects.ts` — Tauri `invoke()` wrappers
    (`createProject`/`listProjects`/`getProject`/`deleteProject`), per
    the "feature-specific service calls live in
    `features/<feature>/services/`" convention from TASK-002.
  - No `hooks/`, `types/` (the `Project`/`CreateProjectInput` types live
    directly in `services/projects.ts`, next to the IPC calls that define
    them), or `tests/` subfolder yet — nothing needs them.
