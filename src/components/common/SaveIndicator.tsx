import { AlertCircle, Check, Loader2 } from "lucide-react";
import { useEffect, useState } from "react";
import { saveStateLabel, type SaveState } from "../../app/useAutoSave";

/**
 * A small floating pill (same shape as the recording dock) that confirms
 * auto-saves. "Saved" fades after a moment; problems stay until resolved.
 */
export function SaveIndicator({ state }: { state: SaveState }) {
  const [visible, setVisible] = useState(false);

  useEffect(() => {
    if (state === "idle") return setVisible(false);
    setVisible(true);
    if (state !== "saved") return;
    const id = window.setTimeout(() => setVisible(false), 1800);
    return () => window.clearTimeout(id);
  }, [state]);

  const label = saveStateLabel(state);
  if (!label) return null;
  const tone = state === "invalid" || state === "error" ? "error" : state === "saved" ? "success" : "busy";
  return (
    <div className={`save-indicator save-indicator--${tone}${visible ? " save-indicator--visible" : ""}`} role="status" aria-live="polite">
      {tone === "error" ? <AlertCircle size={14} /> : tone === "success" ? <Check size={14} /> : <Loader2 size={14} className="save-indicator__spin" />}
      <span>{label}</span>
    </div>
  );
}
