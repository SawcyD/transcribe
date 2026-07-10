import { BookOpenText, Gauge, History, Settings, Sparkles } from "lucide-react";
import { NavLink, Outlet } from "react-router-dom";
import { BRAND } from "../../app/brand";
import { useAppStore } from "../../app/useAppStore";
import { StatusBadge } from "../common/StatusBadge";

const nav = [
  { to: "/", label: "Home", icon: Gauge, end: true },
  { to: "/history", label: "History", icon: History },
  { to: "/dictionary", label: "Dictionary", icon: BookOpenText },
  { to: "/transforms", label: "Transforms", icon: Sparkles },
  { to: "/settings", label: "Settings", icon: Settings },
];

export function AppLayout() {
  const { dictation } = useAppStore();
  return (
    <div className="app-shell">
      <aside className="sidebar">
        <div className="brand-lockup">
          <span className="brand-mark" aria-hidden="true"><i /><i /><i /><i /><i /></span>
          <span><strong>{BRAND.shortName}</strong><small>DEV BUILD</small></span>
        </div>
        <nav aria-label="Primary navigation">
          {nav.map(({ to, label, icon: Icon, end }) => (
            <NavLink key={to} to={to} end={end} className={({ isActive }) => isActive ? "nav-link nav-link--active" : "nav-link"}>
              <Icon size={17} strokeWidth={1.8} />
              <span>{label}</span>
            </NavLink>
          ))}
        </nav>
        <div className="sidebar-status">
          <StatusBadge state={dictation.state} />
          <p>Hold <kbd>Ctrl</kbd> + <kbd>Win</kbd> to dictate. Tap it to finish hands-free.</p>
        </div>
      </aside>
      <main className="content"><Outlet /></main>
    </div>
  );
}
