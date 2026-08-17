import { useEffect, useState } from "react";
import { EmptyState } from "../../components/ui/EmptyState";
import { getErrorMessage } from "../../utils/errorMessage";
import { ProjectList } from "./components/ProjectList";
import { ProjectWorkspace } from "./components/ProjectWorkspace";
import { CreateProjectDialog } from "./components/CreateProjectDialog";
import { DeleteProjectDialog } from "./components/DeleteProjectDialog";
import { listProjects, type Project } from "./services/projects";

// `activeProject` holds the currently open project (Projects list vs.
// Project Workspace) as plain feature-local state — see
// docs/architecture.md ("State management"). It's session-only, not
// persisted, and lives here for the same reason `selectedId` did before
// it: only this feature currently reads/writes it. If a future feature
// needs the current project too, this becomes a same-shape
// `stores/currentProject.ts` slice — a swap, not a rewrite.
type ListState = { state: "loading" } | { state: "ready" } | { state: "error"; message: string };

export function ProjectsView() {
  const [projects, setProjects] = useState<Project[]>([]);
  const [listState, setListState] = useState<ListState>({ state: "loading" });
  const [activeProject, setActiveProject] = useState<Project | null>(null);
  const [createOpen, setCreateOpen] = useState(false);
  const [deleteTarget, setDeleteTarget] = useState<Project | null>(null);

  useEffect(() => {
    void refresh();
  }, []);

  async function refresh() {
    setListState({ state: "loading" });
    try {
      const result = await listProjects();
      setProjects(result);
      setListState({ state: "ready" });
    } catch (error) {
      setListState({ state: "error", message: getErrorMessage(error) });
    }
  }

  function handleCreated(project: Project) {
    setProjects((prev) => [project, ...prev]);
    setActiveProject(project);
    setCreateOpen(false);
  }

  function handleUpdated(updated: Project) {
    // The project just became the most-recently-updated one, so it moves
    // to the top of the local list too — matching the backend's
    // `updated_at DESC` ordering without a full re-fetch.
    setProjects((prev) => [updated, ...prev.filter((project) => project.id !== updated.id)]);
    setActiveProject(updated);
  }

  function handleGone(id: string) {
    setProjects((prev) => prev.filter((project) => project.id !== id));
    setActiveProject(null);
    setDeleteTarget(null);
  }

  if (listState.state === "loading") {
    return <p className="projects-status">Loading projects…</p>;
  }

  if (listState.state === "error") {
    return (
      <EmptyState
        title="Couldn't load projects"
        description={listState.message}
        action={
          <button type="button" className="button" onClick={() => void refresh()}>
            Retry
          </button>
        }
      />
    );
  }

  if (activeProject) {
    return (
      <>
        <ProjectWorkspace
          key={activeProject.id}
          project={activeProject}
          onBack={() => setActiveProject(null)}
          onUpdated={handleUpdated}
          onDeleteRequested={() => setDeleteTarget(activeProject)}
          onGone={() => handleGone(activeProject.id)}
        />
        {deleteTarget && (
          <DeleteProjectDialog
            project={deleteTarget}
            onClose={() => setDeleteTarget(null)}
            onDeleted={handleGone}
          />
        )}
      </>
    );
  }

  if (projects.length === 0) {
    return (
      <>
        <EmptyState
          title="No projects yet"
          description="Create a project to start documenting your processes."
          action={
            <button
              type="button"
              className="button button--primary"
              onClick={() => setCreateOpen(true)}
            >
              + New project
            </button>
          }
        />
        {createOpen && (
          <CreateProjectDialog onClose={() => setCreateOpen(false)} onCreated={handleCreated} />
        )}
      </>
    );
  }

  return (
    <div className="projects-page">
      <div className="projects-toolbar">
        <button
          type="button"
          className="button button--primary"
          onClick={() => setCreateOpen(true)}
        >
          + New project
        </button>
      </div>

      <ProjectList
        projects={projects}
        selectedId={null}
        onSelect={(id) => {
          const project = projects.find((candidate) => candidate.id === id);
          if (project) setActiveProject(project);
        }}
      />

      {createOpen && (
        <CreateProjectDialog onClose={() => setCreateOpen(false)} onCreated={handleCreated} />
      )}
    </div>
  );
}
