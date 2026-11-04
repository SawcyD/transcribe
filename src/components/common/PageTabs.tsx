import type { PageTab } from "./usePageTab";

export function PageTabs<T extends string>({ tabs, active, onSelect, label }: { tabs: ReadonlyArray<PageTab<T>>; active: T; onSelect: (tab: T) => void; label: string }) {
  return (
    <div className="page-tabs" role="tablist" aria-label={label}>
      {tabs.map((tab) => (
        <button
          key={tab.id}
          type="button"
          role="tab"
          aria-selected={tab.id === active}
          className={`page-tabs__tab${tab.id === active ? " page-tabs__tab--active" : ""}`}
          onClick={() => onSelect(tab.id)}
        >
          {tab.label}
        </button>
      ))}
    </div>
  );
}
