import { useEffect, type ReactNode } from "react";
import { native } from "../lib/native";
import { AppStoreProvider } from "./store";

export function AppProviders({ children }: { children: ReactNode }) {
  useEffect(() => {
    void native.settings().then((settings) => {
      document.documentElement.dataset.theme = settings.theme === "system" ? "" : settings.theme;
      document.querySelector(".memora-ui-root")?.setAttribute("data-memora-theme", settings.theme);
    });
  }, []);
  return <AppStoreProvider>{children}</AppStoreProvider>;
}
