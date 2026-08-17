import { formatDate } from "../../../utils/formatDate";
import type { Project } from "../services/projects";

interface ProjectDetailProps {
  project: Project;
  onDelete: () => void;
}

// Clearly-reserved, informational-only placeholders for what a project
// will eventually own (TASK-006+). No fake/functional-looking buttons.
const RESERVED_SECTIONS = ["Processes", "Captures", "Documentation"] as const;

export function ProjectDetail({ project, onDelete }: ProjectDetailProps) {
  return (
    <div className="project-detail">
      <div className="project-detail__header">
        <div>
          <h2 className="project-detail__name">{project.name}</h2>
          {project.description && (
            <p className="project-detail__description">{project.description}</p>
          )}
        </div>
        <button type="button" className="button button--danger" onClick={onDelete}>
          Delete
        </button>
      </div>

      <div className="project-detail__meta">
        <span>Created {formatDate(project.createdAt)}</span>
        <span>Last updated {formatDate(project.updatedAt)}</span>
      </div>

      <div className="project-detail__reserved">
        {RESERVED_SECTIONS.map((label) => (
          <div className="project-detail__section" key={label}>
            <h3 className="project-detail__section-title">{label}</h3>
            <p className="project-detail__section-hint">Not available yet.</p>
          </div>
        ))}
      </div>
    </div>
  );
}
