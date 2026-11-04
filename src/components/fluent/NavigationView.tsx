import { NavigationView as MemoraNavigationView, type NavigationItem as MemoraNavigationItem } from "@memora/ui";
import type { LucideIcon } from "lucide-react";
import { useLocation, useNavigate } from "react-router-dom";

export interface NavigationItem {
  to: string;
  label: string;
  icon: LucideIcon;
  end?: boolean;
}

interface NavigationViewProps {
  items: NavigationItem[];
  footerItems: NavigationItem[];
  collapsed: boolean;
}

function toMemoraItem(item: NavigationItem): MemoraNavigationItem {
  const Icon = item.icon;
  return { id: item.to, label: item.label, icon: <Icon size={16} strokeWidth={1.75} aria-hidden="true" /> };
}

/** Maps the router-aware VoiceFlow navigation onto Memora's keyboard navigation. */
export function NavigationView({ items, footerItems, collapsed }: NavigationViewProps) {
  const navigate = useNavigate();
  const location = useLocation();
  const selectedId = [...items, ...footerItems].find((item) => item.to === location.pathname)?.to ?? "/";

  // The collapse control lives in the sidebar header (AppLayout). The old
  // tooltip wrapped the whole pane, so hovering *anywhere* in the sidebar
  // popped up "Collapse navigation".
  return (
    <MemoraNavigationView
      items={items.map(toMemoraItem)}
      footerItems={footerItems.map(toMemoraItem)}
      selectedId={selectedId}
      collapsed={collapsed}
      onSelect={(to) => navigate(to)}
      ariaLabel="VoiceFlow navigation"
    />
  );
}
