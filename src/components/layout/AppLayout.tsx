import {
  BookOpenText,
  AudioLines,
  Clock,
  Home,
  Info,
  MessageSquare,
  Mic,
  Settings,
  Wand2,
} from "lucide-react";
import { useEffect, useState } from "react";
import { Outlet, useLocation, useNavigate } from "react-router-dom";
import { NavigationView, type NavigationItem } from "../fluent/NavigationView";
import { events, native } from "../../lib/native";

const MAIN_ITEMS: NavigationItem[] = [
  { to: "/", label: "Home", icon: Home, end: true },
  { to: "/dictation", label: "Dictation", icon: Mic },
  { to: "/transforms", label: "Transforms", icon: Wand2 },
  { to: "/dictionary", label: "Dictionary", icon: BookOpenText },
  { to: "/history", label: "History", icon: Clock },
  { to: "/assistant-settings", label: "Assistant", icon: MessageSquare },
];

const FOOTER_ITEMS: NavigationItem[] = [
  { to: "/settings", label: "Settings", icon: Settings },
  { to: "/about", label: "About", icon: Info },
];

/** Width below which the pane collapses to icons, matching NavigationView's compact threshold. */
const COMPACT_BREAKPOINT = 900;

export function AppLayout() {
  const navigate = useNavigate();
  const location = useLocation();
  const [collapsed, setCollapsed] = useState(() => localStorage.getItem("voiceflow.navigation.collapsed") === "true" || window.innerWidth < COMPACT_BREAKPOINT);

  useEffect(() => {
    const onResize = () => setCollapsed(window.innerWidth < COMPACT_BREAKPOINT);
    window.addEventListener("resize", onResize);
    return () => window.removeEventListener("resize", onResize);
  }, []);

  useEffect(() => {
    localStorage.setItem("voiceflow.navigation.collapsed", String(collapsed));
  }, [collapsed]);

  // The tray's Settings item routes the main window from outside the webview.
  useEffect(() => {
    const pending = events.navigate((route) => navigate(route));
    return () => {
      void pending.then((unlisten) => unlisten());
    };
  }, [navigate]);

  // Restore the page the user left off on, then keep it recorded as they navigate.
  useEffect(() => {
    void native.settings().then((settings) => {
      if (settings.lastPage && settings.lastPage !== "/") navigate(settings.lastPage, { replace: true });
    });
    // Runs once: later navigations are persisted by the effect below.
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);

  useEffect(() => {
    void native.settings().then((settings) => {
      if (settings.lastPage === location.pathname) return;
      void native.saveSettings({ ...settings, lastPage: location.pathname });
    });
  }, [location.pathname]);

  return (
    <div className={`app-shell${collapsed ? " app-shell--compact" : ""}`}>
      <aside className="navigation-pane">
        <div className="app-identity" aria-label="VoiceFlow Dev">
          <AudioLines size={16} aria-hidden="true" />
          <strong>VoiceFlow</strong>
        </div>
        <NavigationView items={MAIN_ITEMS} footerItems={FOOTER_ITEMS} collapsed={collapsed} onToggleCollapse={() => setCollapsed((current) => !current)} />
      </aside>
      <main className="content-frame">
        <Outlet />
      </main>
    </div>
  );
}
