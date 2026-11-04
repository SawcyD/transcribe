import { useSearchParams } from "react-router-dom";

export interface PageTab<T extends string> {
  id: T;
  label: string;
}

/**
 * Splits a long settings page into tabs. The active tab lives in the URL
 * (?tab=audio), so links can open a specific tab and it survives reloads.
 */
export function usePageTab<T extends string>(tabs: ReadonlyArray<PageTab<T>>): [T, (tab: T) => void] {
  const [params, setParams] = useSearchParams();
  const requested = params.get("tab");
  const active = tabs.find((tab) => tab.id === requested)?.id ?? tabs[0].id;
  const select = (tab: T) => setParams(tab === tabs[0].id ? {} : { tab }, { replace: true });
  return [active, select];
}
