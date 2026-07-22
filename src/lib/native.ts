import { invoke } from "@tauri-apps/api/core";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import type {
  AppSettings,
  AudioLevelPayload,
  CredentialStatus,
  DashboardStats,
  DictationMode,
  DictationSnapshot,
  DictionaryEntry,
  DictionaryEntryInput,
  TranscriptRecord,
  TransformResponse,
  ScreenContext,
  AssistantDeltaEvent,
  AssistantStateEvent,
  AssistantConversationTurn,
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
  callModeApplication: "Discord",
  callModeOutputDeviceName: null,
  theme: "system",
  buddyStrollEnabled: false,
  buddySpeakResponses: false,
  assistantEndpoint: "https://api.openai.com/v1",
  assistantModel: "gpt-4.1-mini",
  shortcuts: {
    pushToTalk: { modifiers: ["ctrl", "win"], key: null },
    handsFree: { modifiers: ["ctrl", "win"], key: "Space" },
    commandMode: { modifiers: ["ctrl", "alt"], key: "B" },
    cancel: { modifiers: [], key: "Escape" },
  },
  defaultMode: "push_to_talk",
  autoDetectDeveloperApps: true,
  appCleanupStyles: [
    { processName: "code.exe", style: "developer" },
    { processName: "cursor.exe", style: "developer" },
    { processName: "robloxstudiobeta.exe", style: "developer" },
    { processName: "windowsterminal.exe", style: "code_literal" },
    { processName: "powershell.exe", style: "code_literal" },
    { processName: "pwsh.exe", style: "code_literal" },
    { processName: "cmd.exe", style: "code_literal" },
    { processName: "discord.exe", style: "casual" },
  ],
  removeFillerWords: true,
  removeFalseStarts: true,
  backtrackingEnabled: true,
  spokenFormattingEnabled: true,
  voiceActionsEnabled: true,
  showOverlay: true,
  showWaveform: true,
  playTones: true,
  overlayPosition: "bottom_center",
  overlayOpacity: 90,
  buddyEnabled: true,
  buddyShowAtStartup: true,
  buddySize: "medium",
  buddyAlwaysOnTop: true,
  assistantAllowScreenContext: true,
  assistantVoice: null,
  storeRawTranscript: true,
  storeNormalizedTranscript: true,
  storeCleanedTranscript: true,
  includeTranscriptInLogs: false,
  historyRetentionDays: 0,
  maxHistoryEntries: 0,
  confirmPasteAgain: true,
  debugLogging: false,
  closeToTray: true,
  minimizeToTray: false,
  showNotifications: true,
  lastPage: "/",
};

async function call<T>(command: string, args?: Record<string, unknown>, fallback?: T): Promise<T> {
  if (!isTauri()) {
    if (fallback !== undefined) return fallback;
    throw new Error(`${command} requires the VoiceFlow Dev desktop runtime.`);
  }
  try {
    return await invoke<T>(command, args);
  } catch (error) {
    // A WebView can reconnect while the development binary is recompiling or
    // restarting. Read-only calls get a safe in-memory value so a stale native
    // process cannot take down an entire settings page.
    if (fallback !== undefined) return fallback;
    throw error;
  }
}

function normalizeSettings(value: AppSettings | Partial<AppSettings>): AppSettings {
  return {
    ...defaultSettings,
    ...value,
    shortcuts: {
      ...defaultSettings.shortcuts,
      ...(value.shortcuts ?? {}),
    },
    appCleanupStyles: value.appCleanupStyles ?? defaultSettings.appCleanupStyles,
  };
}

export const native = {
  getSnapshot: () => call<DictationSnapshot>("get_dictation_snapshot", undefined, {
    state: "idle",
    interimTranscript: "",
  }),
  start: (mode: DictationMode = "push_to_talk") =>
    call<DictationSnapshot>("start_dictation", { mode }),
  finish: () => call<DictationSnapshot>("finish_dictation"),
  cancel: () => call<DictationSnapshot>("cancel_dictation"),
  history: (query = "") => call<TranscriptRecord[]>("list_transcripts", { query }, []),
  transcript: (id: string) => call<TranscriptRecord>("get_transcript", { id }),
  deleteTranscript: (id: string) => call<void>("delete_transcript", { id }),
  copyText: (text: string) => call<void>("copy_text", { text }),
  pasteTranscript: (id: string) => call<void>("paste_transcript", { id }),
  pasteLatestTranscript: () => call<void>("paste_latest_transcript"),
  applyBuddySettings: () => call<void>("apply_buddy_settings"),
  showHistory: () => call<void>("show_history"),
  showBuddySettings: () => call<void>("show_buddy_settings"),
  openDataFolder: (target: "logs" | "database") => call<void>("open_data_folder", { target }),
  diagnosticReport: () => call<string>("diagnostic_report"),
  clearHistory: () => call<number>("clear_history"),
  settings: async () => normalizeSettings(await call<AppSettings>("get_settings", undefined, defaultSettings)),
  saveSettings: (settings: AppSettings) => call<AppSettings>("save_settings", { settings }),
  credentialStatus: () => call<CredentialStatus>("credential_status", undefined, {
    deepgram: false,
    cleanup: false,
    assistant: false,
  }),
  setCredential: (provider: "deepgram" | "cleanup" | "assistant", secret: string) =>
    call<void>("set_provider_credential", { provider, secret }),
  deleteCredential: (provider: "deepgram" | "cleanup" | "assistant") =>
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
  captureScreenContext: () => call<ScreenContext>("capture_screen_context"),
  hideBuddy: () => call<void>("hide_buddy"),
  openAssistantDrawer: (screenContext: ScreenContext) => call<void>("open_assistant_drawer", { screenContext }),
  pendingAssistantContext: () => call<ScreenContext | null>("get_pending_assistant_context", undefined, null),
  pendingAssistantVoicePrompt: () => call<string | null>("get_pending_assistant_voice_prompt", undefined, null),
  askAssistant: (prompt: string, screenContext: ScreenContext | null, history: AssistantConversationTurn[] = []) => call<string>("ask_assistant", { request: { prompt, screenContext, history } }),
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
  assistantState: (handler: (value: AssistantStateEvent) => void): Promise<UnlistenFn> =>
    isTauri() ? listen<AssistantStateEvent>("assistant-state", ({ payload }) => handler(payload)) : Promise.resolve(() => undefined),
  assistantDelta: (handler: (value: AssistantDeltaEvent) => void): Promise<UnlistenFn> =>
    isTauri() ? listen<AssistantDeltaEvent>("assistant-delta", ({ payload }) => handler(payload)) : Promise.resolve(() => undefined),
  assistantScreenContext: (handler: (value: ScreenContext) => void): Promise<UnlistenFn> =>
    isTauri() ? listen<ScreenContext>("assistant-screen-context", ({ payload }) => handler(payload)) : Promise.resolve(() => undefined),
  /** Route changes requested from outside the webview, e.g. the tray's Settings item. */
  navigate: (handler: (route: string) => void): Promise<UnlistenFn> =>
    isTauri() ? listen<string>("navigate", ({ payload }) => handler(payload)) : Promise.resolve(() => undefined),
  assistantVoicePrompt: (handler: (value: string) => void): Promise<UnlistenFn> =>
    isTauri() ? listen<string>("assistant-voice-prompt", ({ payload }) => handler(payload)) : Promise.resolve(() => undefined),
};
