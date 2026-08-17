import type { AppView, NavItem } from "../../types/navigation";

interface SidebarProps {
  items: NavItem[];
  activeView: AppView;
  onNavigate: (view: AppView) => void;
}

/** Persistent left-hand navigation. Presentation only — the active view
 * and what happens on navigation are owned by the caller. */
export function Sidebar({ items, activeView, onNavigate }: SidebarProps) {
  return (
    <nav aria-label="Main navigation">
      <div className="sidebar__brand">GoLive</div>
      <ul className="sidebar__nav">
        {items.map((item) => (
          <li key={item.id}>
            <button
              type="button"
              className="sidebar__nav-item"
              aria-current={activeView === item.id ? "page" : undefined}
              onClick={() => onNavigate(item.id)}
            >
              {item.label}
            </button>
          </li>
        ))}
      </ul>
    </nav>
  );
}
