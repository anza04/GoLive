/**
 * The set of top-level areas the application shell can navigate between.
 * Deliberately small and flat while the app has only a sidebar + two
 * placeholder pages — see docs/architecture.md ("Navigation") for how this
 * is expected to evolve into real routing.
 */
export type AppView = "projects" | "settings";

export interface NavItem {
  id: AppView;
  label: string;
}
