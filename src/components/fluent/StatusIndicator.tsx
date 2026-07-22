import type { DictationState } from "../../types/models";

type Tone = "idle" | "ready" | "listening" | "processing" | "error";

const TONE_BY_STATE: Partial<Record<DictationState, Tone>> = {
  idle: "ready",
  starting: "listening",
  listening_push_to_talk: "listening",
  listening_hands_free: "listening",
  finalizing_audio: "processing",
  transcribing: "processing",
  cleaning: "processing",
  inserting: "processing",
  completed: "ready",
  cancelled: "ready",
  error: "error",
};

const LABEL: Record<Tone, string> = {
  idle: "Idle",
  ready: "Ready",
  listening: "Listening",
  processing: "Processing",
  error: "Error",
};

interface StatusIndicatorProps {
  state: DictationState;
  /** Overrides the derived label, e.g. "Not configured" before setup. */
  label?: string;
  tone?: Tone;
}

/** Small dot-and-text status readout used on Home and in the navigation footer. */
export function StatusIndicator({ state, label, tone }: StatusIndicatorProps) {
  const resolved = tone ?? TONE_BY_STATE[state] ?? "idle";
  return (
    <span className={`status-indicator status-indicator--${resolved}`}>
      <i aria-hidden="true" />
      {label ?? LABEL[resolved]}
    </span>
  );
}
