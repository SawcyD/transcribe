import { useEffect, useRef, useState } from "react";

export type SaveState = "idle" | "pending" | "saving" | "saved" | "invalid" | "error";

/**
 * Saves `value` shortly after it stops changing, instead of relying on a Save
 * button at the bottom of a long page that is easy to miss.
 *
 * - Nothing is saved until `ready` (the stored value has loaded), and the
 *   first ready value is treated as the baseline, so opening a page never
 *   writes anything.
 * - Invalid values are held back (state "invalid") until they are fixed.
 * - A change still waiting on the debounce is flushed when the page unmounts,
 *   so navigating away right after a toggle does not drop it.
 */
export function useAutoSave<T>(
  value: T,
  { ready, valid, persist, delayMs = 500 }: { ready: boolean; valid: boolean; persist: (value: T) => Promise<unknown>; delayMs?: number },
): SaveState {
  const [state, setState] = useState<SaveState>("idle");
  const baseline = useRef<string | null>(null);
  const pending = useRef<{ value: T; json: string } | null>(null);
  const persistRef = useRef(persist);
  persistRef.current = persist;

  const run = (next: { value: T; json: string }) => {
    pending.current = null;
    setState("saving");
    return persistRef.current(next.value).then(
      () => {
        baseline.current = next.json;
        setState("saved");
      },
      () => setState("error"),
    );
  };

  useEffect(() => {
    if (!ready) return;
    const json = JSON.stringify(value);
    if (baseline.current === null) {
      baseline.current = json;
      return;
    }
    if (json === baseline.current) {
      pending.current = null;
      return;
    }
    if (!valid) {
      pending.current = null;
      setState("invalid");
      return;
    }
    const next = { value, json };
    pending.current = next;
    setState("pending");
    const id = window.setTimeout(() => void run(next), delayMs);
    return () => window.clearTimeout(id);
    // `run` only touches refs and setState.
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [value, ready, valid, delayMs]);

  useEffect(
    () => () => {
      if (pending.current) void persistRef.current(pending.current.value);
    },
    [],
  );

  return state;
}

export function saveStateLabel(state: SaveState): string | null {
  switch (state) {
    case "pending":
    case "saving":
      return "Saving…";
    case "saved":
      return "All changes saved";
    case "invalid":
      return "Fix the highlighted setting to save";
    case "error":
      return "Couldn't save — try again";
    default:
      return null;
  }
}
