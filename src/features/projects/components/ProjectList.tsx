import { formatDate } from "../../../utils/formatDate";
import type { Project } from "../services/projects";

interface ProjectListProps {
  projects: Project[];
  selectedId: string | null;
  onSelect: (id: string) => void;
}

export function ProjectList({ projects, selectedId, onSelect }: ProjectListProps) {
  return (
    <ul className="project-list">
      {projects.map((project) => (
        <li key={project.id}>
          <button
            type="button"
            className="project-list-item"
            aria-current={project.id === selectedId ? "true" : undefined}
            onClick={() => onSelect(project.id)}
          >
            <span className="project-list-item__name">{project.name}</span>
            <span className="project-list-item__meta">
              Updated {formatDate(project.updatedAt)}
            </span>
          </button>
        </li>
      ))}
    </ul>
  );
}
