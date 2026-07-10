import { createContext } from "react";
import type { AppStoreState } from "./state";

export interface StoreValue extends AppStoreState {
  refreshLastTranscript: () => Promise<void>;
}

export const AppStoreContext = createContext<StoreValue | null>(null);
