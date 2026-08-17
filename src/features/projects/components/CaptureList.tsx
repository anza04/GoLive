import { formatDate } from "../../../utils/formatDate";
import { CaptureTypeBadge } from "./CaptureTypeBadge";
import type { Capture } from "../services/captures";

interface CaptureListProps {
  captures: Capture[];
  selectedId: string | null;
  onSelect: (id: string) => void;
}

export function CaptureList({ captures, selectedId, onSelect }: CaptureListProps) {
  return (
    <ul className="project-list">
      {captures.map((capture) => (
        <li key={capture.id}>
          <button
            type="button"
            className="project-list-item"
            aria-current={capture.id === selectedId ? "true" : undefined}
            onClick={() => onSelect(capture.id)}
          >
            <span className="project-list-item__name">{capture.title}</span>
            <span className="project-list-item__meta">
              <CaptureTypeBadge type={capture.type} />
              <span>Updated {formatDate(capture.updatedAt)}</span>
            </span>
          </button>
        </li>
      ))}
    </ul>
  );
}
