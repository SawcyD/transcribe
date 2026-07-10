import type { AudioLevelPayload, DictationSnapshot, TranscriptRecord } from "../types/models";

export interface AppStoreState {
  dictation: DictationSnapshot;
  audio: AudioLevelPayload | null;
  lastTranscript: TranscriptRecord | null;
}

export type AppStoreAction =
  | { type: "snapshot"; value: DictationSnapshot }
  | { type: "audio"; value: AudioLevelPayload }
  | { type: "last-transcript"; value: TranscriptRecord | null };

export const initialAppState: AppStoreState = {
  dictation: { state: "idle", interimTranscript: "" },
  audio: null,
  lastTranscript: null,
};

export function appReducer(state: AppStoreState, action: AppStoreAction): AppStoreState {
  switch (action.type) {
    case "snapshot":
      return { ...state, dictation: action.value, audio: action.value.state === "idle" ? null : state.audio };
    case "audio":
      return { ...state, audio: action.value };
    case "last-transcript":
      return { ...state, lastTranscript: action.value };
  }
}
