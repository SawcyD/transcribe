import { invoke } from "@tauri-apps/api/core";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import type {
  AppSettings,
  AudioLevelPayload,
  CredentialStatus,
  DashboardStats,
  DictationSnapshot,
  DictionaryEntry,
  DictionaryEntryInput,
  TranscriptRecord,
  TransformResponse,
} from "../types/models";

export const isTauri = (): boolean => "__TAURI_INTERNALS__" in window;

export const defaultSettings: AppSettings = {
  microphoneName: null,
  transcriptionProvider: "deepgram",
  transcriptionModel: "nova-3",
  language: "en-US",
  cleanupEnabled: true,
  cleanupEndpoint: "https://api.openai.com/v1",
  cleanupModel: "gpt-4.1-mini",
  cleanupStyle: "balanced",
  autoApplyTransform: null,
  pasteDelayMs: 140,
  restoreClipboard: true,
  pressEnterEnabled: false,
  saveHistory: true,
  saveAudio: false,
  sessionLimitMinutes: 20,
  noiseFloorDb: -52,
};

async function call<T>(command: string, args?: Record<string, unknown>, fallback?: T): Promise<T> {
  if (!isTauri()) {
    if (fallback !== undefined) return fallback;
    throw new Error(`${command} requires the VoiceFlow Dev desktop runtime.`);
  }
  return invoke<T>(command, args);
}

export const native = {
  getSnapshot: () => call<DictationSnapshot>("get_dictation_snapshot", undefined, {
    state: "idle",
    interimTranscript: "",
  }),
  start: (mode: "push_to_talk" | "hands_free" = "push_to_talk") =>
    call<DictationSnapshot>("start_dictation", { mode }),
  finish: () => call<DictationSnapshot>("finish_dictation"),
  cancel: () => call<DictationSnapshot>("cancel_dictation"),
  history: (query = "") => call<TranscriptRecord[]>("list_transcripts", { query }, []),
  transcript: (id: string) => call<TranscriptRecord>("get_transcript", { id }),
  deleteTranscript: (id: string) => call<void>("delete_transcript", { id }),
  copyText: (text: string) => call<void>("copy_text", { text }),
  pasteTranscript: (id: string) => call<void>("paste_transcript", { id }),
  settings: () => call<AppSettings>("get_settings", undefined, defaultSettings),
  saveSettings: (settings: AppSettings) => call<AppSettings>("save_settings", { settings }),
  credentialStatus: () => call<CredentialStatus>("credential_status", undefined, {
    deepgram: false,
    cleanup: false,
  }),
  setCredential: (provider: "deepgram" | "cleanup", secret: string) =>
    call<void>("set_provider_credential", { provider, secret }),
  deleteCredential: (provider: "deepgram" | "cleanup") =>
    call<void>("delete_provider_credential", { provider }),
  microphones: () => call<string[]>("list_microphones", undefined, ["System default microphone"]),
  dictionary: () => call<DictionaryEntry[]>("list_dictionary_entries", undefined, []),
  saveDictionaryEntry: (id: string | null, entry: DictionaryEntryInput) => call<DictionaryEntry>("save_dictionary_entry", { id, entry }),
  deleteDictionaryEntry: (id: string) => call<void>("delete_dictionary_entry", { id }),
  stats: () => call<DashboardStats>("dashboard_stats", undefined, {
    dailyWords: 0,
    dailySessions: 0,
    estimatedMinutesSaved: 0,
  }),
  transform: (text: string, transformId: string) => call<TransformResponse>("transform_text", {
    request: { text, transformId },
  }),
};

export const events = {
  state: (handler: (value: DictationSnapshot) => void): Promise<UnlistenFn> =>
    isTauri()
      ? listen<DictationSnapshot>("dictation-state", ({ payload }) => handler(payload))
      : Promise.resolve(() => undefined),
  audio: (handler: (value: AudioLevelPayload) => void): Promise<UnlistenFn> =>
    isTauri()
      ? listen<AudioLevelPayload>("audio-level", ({ payload }) => handler(payload))
      : Promise.resolve(() => undefined),
};
