import { formatDate } from "../../../utils/formatDate";
import type { Project } from "../services/projects";

interface ProjectOverviewProps {
  project: Project;
}

// Informational-only preview of what a project still doesn't have real
// functionality for. "Processes" was removed from this list in TASK-007
// — it's now a real, working workspace tab, not a placeholder; "Captures"
// was removed the same way here (bugfix — see DECISIONS.md, "hierarchy
// clarity"): captures have been fully working since TASK-008/009, just
// nested under a selected Process rather than surfaced on this tab, and
// this tile telling users they're "not available yet" actively hid a
// feature the app already has. No clickable/functional-looking controls
// here.
const FUTURE_SECTIONS = [
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
