import { NavigationView as MemoraNavigationView, Tooltip, type NavigationItem as MemoraNavigationItem } from "@memora/ui";
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
  onToggleCollapse?: () => void;
}

function toMemoraItem(item: NavigationItem): MemoraNavigationItem {
  const Icon = item.icon;
  return { id: item.to, label: item.label, icon: <Icon size={16} strokeWidth={1.75} aria-hidden="true" /> };
}

/** Maps the router-aware VoiceFlow navigation onto Memora's keyboard navigation. */
export function NavigationView({ items, footerItems, collapsed, onToggleCollapse }: NavigationViewProps) {
  const navigate = useNavigate();
  const location = useLocation();
  const selectedId = [...items, ...footerItems].find((item) => item.to === location.pathname)?.to ?? "/";

  return (
    <Tooltip content={collapsed ? "Expand navigation" : "Collapse navigation"} placement="right">
      <div>
        <MemoraNavigationView
          items={items.map(toMemoraItem)}
          footerItems={footerItems.map(toMemoraItem)}
          selectedId={selectedId}
          collapsed={collapsed}
          onToggleCollapse={onToggleCollapse}
          onSelect={(to) => navigate(to)}
          ariaLabel="VoiceFlow navigation"
        />
      </div>
    </Tooltip>
  );
}
