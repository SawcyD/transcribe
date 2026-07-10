import { useContext } from "react";
import { AppStoreContext, type StoreValue } from "./storeContext";

export function useAppStore(): StoreValue {
  const value = useContext(AppStoreContext);
  if (!value) throw new Error("useAppStore must be used inside AppStoreProvider");
  return value;
}
