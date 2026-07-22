import type { DictationMode, DictationState } from "../types/models";

export type BuddyMood = "idle" | "waving" | "sleeping" | "walking" | "push_to_talk" | "hands_free" | "call_recording" | "capturing" | "thinking" | "analyzing" | "success" | "warning";
export type CaptureMood = "capturing" | "captured" | "failed" | null;

export const buddyFrames: Record<BuddyMood, number[]> = {
  idle: Array.from({ length: 30 }, (_, frame) => frame),
  waving: [0, 1, 2, 3],
  sleeping: Array.from({ length: 32 }, (_, frame) => frame),
  walking: Array.from({ length: 8 }, (_, frame) => frame),
  push_to_talk: [3],
  hands_free: [4],
  call_recording: [0, 1, 2, 3],
  capturing: [5],
  thinking: [6],
  analyzing: [7],
  success: [8, 9],
  warning: [10],
};

export function buddyMoodFor(
  dictation: DictationState,
  capture: CaptureMood,
  resting: boolean,
  mode?: DictationMode,
): BuddyMood {
  if (capture === "capturing") return "capturing";
  if (capture === "captured") return "success";
  if (capture === "failed") return "warning";
  if (dictation === "listening_push_to_talk") return "push_to_talk";
  if (dictation === "listening_hands_free") return mode === "call" ? "call_recording" : "hands_free";
  if (dictation === "starting" || dictation === "finalizing_audio") return "thinking";
  if (dictation === "transcribing" || dictation === "cleaning" || dictation === "inserting") return "analyzing";
  if (dictation === "completed") return "success";
  if (dictation === "error") return "warning";
  return resting ? "sleeping" : "idle";
}
