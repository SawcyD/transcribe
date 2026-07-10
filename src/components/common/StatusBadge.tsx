import type { DictationState } from "../../types/models";

const labels: Record<DictationState, string> = {
  idle: "Ready",
  starting: "Starting",
  listening_push_to_talk: "Listening",
  listening_hands_free: "Hands-free",
  finalizing_audio: "Finalizing",
  transcribing: "Transcribing",
  cleaning: "Cleaning",
  inserting: "Inserting",
  completed: "Inserted",
  cancelled: "Cancelled",
  error: "Needs attention",
};

export function StatusBadge({ state }: { state: DictationState }) {
  const tone = state === "error" ? "danger" : state === "idle" || state === "completed" ? "success" : "active";
  return <span className={`status status--${tone}`}><i aria-hidden="true" />{labels[state]}</span>;
}
