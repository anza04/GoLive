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
// "overview" and "processes" (TASK-007) have real content. "captures"
// and "documentation" are both disabled — but neither is a stand-in for
// a genuinely missing feature; both are fully working one level deeper,
// inside a selected Process, not here (Captures since TASK-008/009,
// Word export since TASK-020's "Export to Word" button in a Process's
// "Process draft" section). Both `hint`s say so explicitly rather than
// using the generic "not available yet" every other disabled tab in
// this app uses — a deliberate bugfix, not an oversight (see
// DECISIONS.md, "hierarchy clarity"): a generic tooltip here would
// imply the feature doesn't exist anywhere in the app at all. This tab
// stays reserved for a possible future *project-wide* aggregated view
// of either — not today's real, per-Process ones.
const WORKSPACE_TABS: { id: WorkspaceTabId; label: string; available: boolean; hint?: string }[] = [
  { id: "overview", label: "Overview", available: true },
  { id: "processes", label: "Processes", available: true },
  {
    id: "captures",
    label: "Captures",
    available: false,
    hint: "Open a process under Processes to view and add its captures",
  },
  {
    id: "documentation",
    label: "Documentation",
    available: false,
    hint: "Open a process under Processes, then use \"Export to Word\" in its Process draft section",
  },
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
        <div className="entity-header__titles">
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
            title={!tab.available ? (tab.hint ?? "Not available yet") : undefined}
            onClick={() => setActiveTab(tab.id)}
          >
            {tab.label}
          </button>
        ))}
      </nav>

      <div className="workspace-content">
        {activeTab === "overview" && <ProjectOverview project={project} />}
        {activeTab === "processes" && (
          <ProcessesView projectId={project.id} projectName={project.name} />
        )}
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
