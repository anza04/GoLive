import { ProjectsView } from "../features/projects/ProjectsView";

/** Routed entry point for the Projects area. All real state/logic lives
 * in the `projects` feature — this page only composes it (see
 * docs/architecture.md, "Current frontend structure"). */
export function ProjectsPage() {
  return <ProjectsView />;
}
