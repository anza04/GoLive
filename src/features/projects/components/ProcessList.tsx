import { formatDate } from "../../../utils/formatDate";
import { ProcessStatusBadge } from "./ProcessStatusBadge";
import type { Process } from "../services/processes";

interface ProcessListProps {
  processes: Process[];
  selectedId: string | null;
  onSelect: (id: string) => void;
}

export function ProcessList({ processes, selectedId, onSelect }: ProcessListProps) {
  return (
    <ul className="project-list">
      {processes.map((process) => (
        <li key={process.id}>
          <button
            type="button"
            className="project-list-item"
            aria-current={process.id === selectedId ? "true" : undefined}
            onClick={() => onSelect(process.id)}
          >
            <span className="project-list-item__name">{process.name}</span>
            <span className="project-list-item__meta">
              <ProcessStatusBadge status={process.status} />
              <span>Updated {formatDate(process.updatedAt)}</span>
            </span>
          </button>
        </li>
      ))}
    </ul>
  );
}
