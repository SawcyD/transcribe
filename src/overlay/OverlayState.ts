import type { DictationState } from "../types/models";

export interface OverlayPresentation {
  label: string;
  tone: "listening" | "processing" | "success" | "error";
  showFinish: boolean;
}

export function overlayPresentation(state: DictationState): OverlayPresentation {
  if (state === "listening_push_to_talk") return { label: "Listening", tone: "listening", showFinish: false };
  if (state === "listening_hands_free") return { label: "Listening", tone: "listening", showFinish: true };
  if (state === "completed") return { label: "Inserted", tone: "success", showFinish: false };
  if (state === "error") return { label: "Failed", tone: "error", showFinish: false };
  if (state === "starting") return { label: "Starting", tone: "processing", showFinish: false };
  return { label: "Processing", tone: "processing", showFinish: false };
}
