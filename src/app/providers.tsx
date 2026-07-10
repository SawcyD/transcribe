import type { ReactNode } from "react";
import { AppStoreProvider } from "./store";

export function AppProviders({ children }: { children: ReactNode }) {
  return <AppStoreProvider>{children}</AppStoreProvider>;
}
