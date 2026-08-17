# features

Feature-scoped modules (e.g. projects, captures, processes, settings), each
owning the code that only that feature needs: components, services
(Tauri calls), hooks, types, local state.

**Start simple, split only when complexity justifies it.** A new feature
starts as a single file or a small flat folder. Only break it into
`components/`, `services/`, `hooks/`, `types/`, `tests/` subfolders once it
actually has enough of each to need the separation — don't scaffold empty
subfolders in advance.

Not yet populated — introduced starting with the Projects feature (M1).
