import { useEffect, useState, type ReactNode } from "react";
import { AppShell } from "./components/layout/AppShell";
import { Sidebar } from "./components/layout/Sidebar";
import { Header } from "./components/layout/Header";
import { ProjectsPage } from "./pages/ProjectsPage";
import { SettingsPage } from "./pages/SettingsPage";
import { checkFoundationStatus } from "./services/foundation";
import { ActiveProcessProvider } from "./stores/activeProcess";
import type { AppView, NavItem } from "./types/navigation";
import "./App.css";

const NAV_ITEMS: NavItem[] = [
  { id: "projects", label: "Projects" },
  { id: "settings", label: "Settings" },
];

// Maps each view to the page that renders it. This is the navigation
// convention this task establishes: switching to a real router later
// (e.g. react-router) means turning each entry here into a
// <Route path="/..." element={<... />} />, and replacing `activeView` /
// `setActiveView` with the router's own location — everything else
// (Sidebar, Header, AppShell) stays the same.
const PAGES: Record<AppView, () => ReactNode> = {
  projects: ProjectsPage,
  settings: SettingsPage,
};

type BackendStatus =
  | { state: "checking" }
  | { state: "ok"; message: string }
  | { state: "error"; message: string };

function App() {
  const [activeView, setActiveView] = useState<AppView>("projects");
  const [status, setStatus] = useState<BackendStatus>({ state: "checking" });

  useEffect(() => {
    checkFoundationStatus()
      .then((message) => setStatus({ state: "ok", message }))
      .catch((error) =>
        setStatus({ state: "error", message: String(error) }),
      );
  }, []);

  const activeLabel =
    NAV_ITEMS.find((item) => item.id === activeView)?.label ?? "";
  const ActivePage = PAGES[activeView];
  const statusDetail = status.state !== "checking" ? status.message : undefined;

  return (
    <ActiveProcessProvider>
      <AppShell
        sidebar={
          <Sidebar
            items={NAV_ITEMS}
            activeView={activeView}
            onNavigate={setActiveView}
          />
        }
        header={
          <Header
            title={activeLabel}
            status={status.state}
            statusDetail={statusDetail}
          />
        }
      >
        <ActivePage />
      </AppShell>
    </ActiveProcessProvider>
  );
}

export default App;
