import { useState } from "react";
import { ProjectOverview } from "./ProjectOverview";
import { EditProjectDialog } from "./EditProjectDialog";
import type { Project } from "../services/projects";

interface ProjectWorkspaceProps {
  project: Project;
  onBack: () => void;
  onUpdated: (project: Project) => void;
  onDeleteRequested: () => void;
  /** The project no longer exists (e.g. deleted elsewhere) — leave the
   * workspace and drop it from the list. */
  onGone: () => void;
}

// The workspace's navigation structure for what a project will eventually
// contain. Only "overview" has content today; the rest are reserved,
// genuinely disabled (not fake-clickable) entries — see
// docs/architecture.md for how this maps onto real routes later, the
// same mechanical swap already documented for the top-level Sidebar.
const WORKSPACE_TABS = [
  { id: "overview", label: "Overview" },
  { id: "processes", label: "Processes" },
  { id: "captures", label: "Captures" },
  { id: "documentation", label: "Documentation" },
] as const;

const AVAILABLE_TAB = "overview";

export function ProjectWorkspace({
  project,
  onBack,
  onUpdated,
  onDeleteRequested,
  onGone,
}: ProjectWorkspaceProps) {
  const [editOpen, setEditOpen] = useState(false);

  return (
    <div className="workspace">
      <button type="button" className="workspace__back" onClick={onBack}>
        ← Projects
      </button>

      <div className="workspace__header">
        <div>
          <h2 className="workspace__name">{project.name}</h2>
          {project.description && (
            <p className="workspace__description">{project.description}</p>
          )}
        </div>
        <div className="workspace__actions">
          <button type="button" className="button" onClick={() => setEditOpen(true)}>
            Edit
          </button>
          <button type="button" className="button button--danger" onClick={onDeleteRequested}>
            Delete
          </button>
        </div>
      </div>

      <nav className="workspace-tabs" aria-label="Project workspace areas">
        {WORKSPACE_TABS.map((tab) => (
          <button
            key={tab.id}
            type="button"
            className="workspace-tabs__item"
            aria-current={tab.id === AVAILABLE_TAB ? "page" : undefined}
            disabled={tab.id !== AVAILABLE_TAB}
            title={tab.id !== AVAILABLE_TAB ? "Not available yet" : undefined}
          >
            {tab.label}
          </button>
        ))}
      </nav>

      <div className="workspace-content">
        <ProjectOverview project={project} />
      </div>

      {editOpen && (
        <EditProjectDialog
          project={project}
          onClose={() => setEditOpen(false)}
          onUpdated={(updated) => {
            setEditOpen(false);
            onUpdated(updated);
          }}
          onNotFound={onGone}
        />
      )}
    </div>
  );
}
