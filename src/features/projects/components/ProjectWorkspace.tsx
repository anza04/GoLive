import { useState } from "react";
import { ProjectOverview } from "./ProjectOverview";
import { ProcessesView } from "./ProcessesView";
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

type WorkspaceTabId = "overview" | "processes" | "captures" | "documentation";

// The workspace's navigation structure for what a project contains.
// "overview" and "processes" (TASK-007) have real content; "captures" and
// "documentation" are reserved, genuinely disabled (not fake-clickable)
// entries — see docs/architecture.md for how this maps onto real routes
// later, the same mechanical swap already documented for the top-level
// Sidebar.
const WORKSPACE_TABS: { id: WorkspaceTabId; label: string; available: boolean }[] = [
  { id: "overview", label: "Overview", available: true },
  { id: "processes", label: "Processes", available: true },
  { id: "captures", label: "Captures", available: false },
  { id: "documentation", label: "Documentation", available: false },
];

export function ProjectWorkspace({
  project,
  onBack,
  onUpdated,
  onDeleteRequested,
  onGone,
}: ProjectWorkspaceProps) {
  const [editOpen, setEditOpen] = useState(false);
  const [activeTab, setActiveTab] = useState<WorkspaceTabId>("overview");

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
            aria-current={tab.id === activeTab ? "page" : undefined}
            disabled={!tab.available}
            title={!tab.available ? "Not available yet" : undefined}
            onClick={() => setActiveTab(tab.id)}
          >
            {tab.label}
          </button>
        ))}
      </nav>

      <div className="workspace-content">
        {activeTab === "overview" && <ProjectOverview project={project} />}
        {activeTab === "processes" && <ProcessesView projectId={project.id} />}
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
