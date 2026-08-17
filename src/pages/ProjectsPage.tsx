import { EmptyState } from "../components/ui/EmptyState";

/** Placeholder Projects area. No project data exists yet — see
 * PROJECT_STATE.md for what's still pending (SQLite, project CRUD). */
export function ProjectsPage() {
  return (
    <EmptyState
      title="No projects yet"
      description="Your GoLive projects will appear here."
      action={
        <button
          type="button"
          className="button"
          disabled
          title="Project creation isn't available yet"
        >
          New Project
        </button>
      }
    />
  );
}
