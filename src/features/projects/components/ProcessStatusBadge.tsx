import type { ProcessStatus } from "../services/processes";

interface ProcessStatusBadgeProps {
  status: ProcessStatus;
}

// Internal enum formatting (draft / in_progress / completed) is never
// shown to the user directly — always through this label map.
const STATUS_LABEL: Record<ProcessStatus, string> = {
  draft: "Draft",
  in_progress: "In progress",
  completed: "Completed",
};

export function ProcessStatusBadge({ status }: ProcessStatusBadgeProps) {
  return <span className={`status-badge status-badge--${status}`}>{STATUS_LABEL[status]}</span>;
}
