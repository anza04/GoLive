import { formatDate } from "../../../utils/formatDate";
import type { Project } from "../services/projects";

interface ProjectOverviewProps {
  project: Project;
}

// Informational-only preview of what a project will eventually own
// (TASK-007+). No clickable/functional-looking controls.
const FUTURE_SECTIONS = [
  { title: "Processes", description: "Document the processes collected from users." },
  { title: "Captures", description: "Review screenshots and recordings." },
  { title: "Documentation", description: "Generate the final process documentation." },
] as const;

export function ProjectOverview({ project }: ProjectOverviewProps) {
  return (
    <div className="project-overview">
      <div className="project-overview__meta">
        <span>Created {formatDate(project.createdAt)}</span>
        <span>Last updated {formatDate(project.updatedAt)}</span>
      </div>

      <div className="reserved-sections">
        {FUTURE_SECTIONS.map((section) => (
          <div className="reserved-section" key={section.title}>
            <h3 className="reserved-section__title">{section.title}</h3>
            <p className="reserved-section__hint">{section.description}</p>
          </div>
        ))}
      </div>
    </div>
  );
}
