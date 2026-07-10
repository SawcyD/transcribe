import { useCallback, useEffect, useMemo, useReducer, type ReactNode } from "react";
import { events, native } from "../lib/native";
import { appReducer, initialAppState } from "./state";
import { AppStoreContext } from "./storeContext";

export function AppStoreProvider({ children }: { children: ReactNode }) {
  const [state, dispatch] = useReducer(appReducer, initialAppState);

  const refreshLastTranscript = useCallback(async () => {
    const records = await native.history();
    dispatch({ type: "last-transcript", value: records.at(0) ?? null });
  }, []);

  useEffect(() => {
    let disposed = false;
    const unlisteners: Array<() => void> = [];
    void native.getSnapshot().then((value) => !disposed && dispatch({ type: "snapshot", value }));
    void refreshLastTranscript();
    void events.state((value) => {
      if (disposed) return;
      dispatch({ type: "snapshot", value });
      if (value.state === "completed" || value.state === "error") void refreshLastTranscript();
    }).then((unlisten) => unlisteners.push(unlisten));
    void events.audio((value) => {
      if (!disposed) dispatch({ type: "audio", value });
    }).then((unlisten) => unlisteners.push(unlisten));
    return () => {
      disposed = true;
      unlisteners.forEach((unlisten) => unlisten());
    };
  }, [refreshLastTranscript]);

  const value = useMemo(() => ({ ...state, refreshLastTranscript }), [state, refreshLastTranscript]);
  return <AppStoreContext.Provider value={value}>{children}</AppStoreContext.Provider>;
}
