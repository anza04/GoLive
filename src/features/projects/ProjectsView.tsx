import { useEffect, useState } from "react";
import { EmptyState } from "../../components/ui/EmptyState";
import { getErrorMessage } from "../../utils/errorMessage";
import { ProjectList } from "./components/ProjectList";
import { ProjectDetail } from "./components/ProjectDetail";
import { CreateProjectDialog } from "./components/CreateProjectDialog";
import { DeleteProjectDialog } from "./components/DeleteProjectDialog";
import { listProjects, type Project } from "./services/projects";

// Selected-project state lives here, at the feature root, since only this
// feature currently needs it — see docs/architecture.md ("State
// management"). If a future feature (e.g. a floating widget) needs to
// know the current project too, this `useState<string | null>` moves
// into a `stores/currentProject.ts` slice with the same shape; nothing
// else here would need to change.
type ListState = { state: "loading" } | { state: "ready" } | { state: "error"; message: string };

export function ProjectsView() {
  const [projects, setProjects] = useState<Project[]>([]);
  const [listState, setListState] = useState<ListState>({ state: "loading" });
  const [selectedId, setSelectedId] = useState<string | null>(null);
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
    setSelectedId(project.id);
    setCreateOpen(false);
  }

  function handleDeleted(id: string) {
    setProjects((prev) => prev.filter((project) => project.id !== id));
    setSelectedId((current) => (current === id ? null : current));
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

  const selectedProject = projects.find((project) => project.id === selectedId) ?? null;

  return (
    <div className="projects-layout">
      <div className="projects-list-pane">
        <div className="projects-toolbar">
          <button
            type="button"
            className="button button--primary"
            onClick={() => setCreateOpen(true)}
          >
            + New project
          </button>
        </div>
        <ProjectList projects={projects} selectedId={selectedId} onSelect={setSelectedId} />
      </div>

      <div className="projects-detail-pane">
        {selectedProject ? (
          <ProjectDetail
            project={selectedProject}
            onDelete={() => setDeleteTarget(selectedProject)}
          />
        ) : (
          <EmptyState
            title="No project selected"
            description="Select a project from the list to see its details."
          />
        )}
      </div>

      {createOpen && (
        <CreateProjectDialog onClose={() => setCreateOpen(false)} onCreated={handleCreated} />
      )}
      {deleteTarget && (
        <DeleteProjectDialog
          project={deleteTarget}
          onClose={() => setDeleteTarget(null)}
          onDeleted={handleDeleted}
        />
      )}
    </div>
  );
}
