import type { CaptureType } from "../services/captures";

interface CaptureTypeBadgeProps {
  type: CaptureType;
}

// Internal enum formatting (screenshot / recording / note) is never shown
// to the user directly — always through this label map.
const TYPE_LABEL: Record<CaptureType, string> = {
  screenshot: "Screenshot",
  recording: "Recording",
  note: "Note",
};

export function CaptureTypeBadge({ type }: CaptureTypeBadgeProps) {
  return <span className={`capture-type-badge capture-type-badge--${type}`}>{TYPE_LABEL[type]}</span>;
}
