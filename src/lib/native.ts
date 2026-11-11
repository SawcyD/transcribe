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
  AssistantChatSummary,
  AssistantChatTurn,
} from "../types/models";

export type AgentState = "connecting" | "listening" | "thinking" | "speaking" | "ended" | "error";

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
  buddySpeakResponses: false,
  assistantEndpoint: "https://api.openai.com/v1",
  assistantModel: "gpt-4.1-mini",
  shortcuts: {
    pushToTalk: { modifiers: ["ctrl", "win"], key: null },
    handsFree: { modifiers: ["ctrl", "win"], key: "Space" },
    commandMode: { modifiers: ["ctrl", "alt"], key: "B" },
    voiceAgent: { modifiers: ["ctrl", "alt"], key: "V" },
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
  cleanupMinWords: 8,
  keepTranscriptionWarm: true,
  assistantRememberChats: true,
  voiceAgentInterruptible: false,
  outputDeviceName: null,
  backtrackingEnabled: true,
  spokenFormattingEnabled: true,
  voiceActionsEnabled: true,
  showOverlay: true,
  showWaveform: true,
  playTones: true,
  overlayPosition: "bottom_center",
  overlayOpacity: 90,
  assistantAllowScreenContext: true,
  assistantVoice: null,
  voiceAgentVoice: "flux-kit-en",
  voiceAgentExpressivity: 1,
  voiceAgentPersonality: "friendly",
  voiceAgentCanAct: true,
  wakeWordEnabled: false,
  numerals: false,
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
  /** Reads text aloud on the chosen output device (plays in the native layer). */
  speakText: (text: string) => call<void>("speak_text", { text }),
  stopSpeaking: () => call<void>("stop_speaking", undefined, undefined),
  playTone: (name: "start" | "stop" | "success" | "error" | "test") => call<void>("play_tone", { name }),
  testOutputDevice: (device: string | null) => call<void>("test_output_device", { device }),
  outputDevices: () => call<string[]>("list_output_devices", undefined, []),
  setTranscriptFavorite: (id: string, favorite: boolean) => call<void>("set_transcript_favorite", { id, favorite }),
  copyText: (text: string) => call<void>("copy_text", { text }),
  pasteTranscript: (id: string) => call<void>("paste_transcript", { id }),
  pasteLatestTranscript: () => call<void>("paste_latest_transcript"),
  showHistory: () => call<void>("show_history"),
  showWhiteboard: () => call<void>("show_whiteboard"),
  openFlowProfile: () => call<void>("open_flow_profile"),
  listWhiteboards: () => call<string[]>("list_whiteboards", undefined, []),
  loadWhiteboard: (name: string) => call<string | null>("load_whiteboard", { name }, null),
  saveWhiteboard: (name: string, content: string) => call<void>("save_whiteboard", { name, content }),
  openDataFolder: (target: "logs" | "database") => call<void>("open_data_folder", { target }),
  diagnosticReport: () => call<string>("diagnostic_report"),
  clearHistory: () => call<number>("clear_history"),
  settings: async () => normalizeSettings(await call<AppSettings>("get_settings", undefined, defaultSettings)),
  saveSettings: (settings: AppSettings) => call<AppSettings>("save_settings", { settings }),
  credentialStatus: () => call<CredentialStatus>("credential_status", undefined, {
    deepgram: false,
    cleanup: false,
    assistant: false,
    search: false,
  }),
  setCredential: (provider: "deepgram" | "cleanup" | "assistant" | "search", secret: string) =>
    call<void>("set_provider_credential", { provider, secret }),
  deleteCredential: (provider: "deepgram" | "cleanup" | "assistant" | "search") =>
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
  openAssistantDrawer: (screenContext: ScreenContext) => call<void>("open_assistant_drawer", { screenContext }),
  pendingAssistantContext: () => call<ScreenContext | null>("get_pending_assistant_context", undefined, null),
  pendingAssistantVoicePrompt: () => call<string | null>("get_pending_assistant_voice_prompt", undefined, null),
  /** Starts a streamed answer; resolves to the request id (not the answer). */
  askAssistant: (prompt: string, screenContext: ScreenContext | null, history: AssistantConversationTurn[] = [], chatId: string | null = null) =>
    call<string>("ask_assistant", { request: { prompt, screenContext, history, chatId } }),
  startVoiceAgent: () => call<void>("start_voice_agent"),
  stopVoiceAgent: () => call<void>("stop_voice_agent"),
  assistantChats: () => call<AssistantChatSummary[]>("list_assistant_chats", undefined, []),
  assistantChat: (id: string) => call<AssistantChatTurn[]>("get_assistant_chat", { id }, []),
  deleteAssistantChat: (id: string) => call<void>("delete_assistant_chat", { id }),
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
  assistantVoicePrompt: (handler: (value: { prompt: string; screenContext: ScreenContext | null }) => void): Promise<UnlistenFn> =>
    isTauri()
      ? listen<{ prompt: string; screenContext: ScreenContext | null }>("assistant-voice-prompt", ({ payload }) => handler(payload))
      : Promise.resolve(() => undefined),
  agentState: (handler: (value: { state: AgentState; message: string | null }) => void): Promise<UnlistenFn> =>
    isTauri() ? listen<{ state: AgentState; message: string | null }>("agent-state", ({ payload }) => handler(payload)) : Promise.resolve(() => undefined),
  agentTranscript: (handler: (value: { role: string; content: string }) => void): Promise<UnlistenFn> =>
    isTauri() ? listen<{ role: string; content: string }>("agent-transcript", ({ payload }) => handler(payload)) : Promise.resolve(() => undefined),
  agentLevel: (handler: (value: number) => void): Promise<UnlistenFn> =>
    isTauri() ? listen<number>("agent-level", ({ payload }) => handler(payload)) : Promise.resolve(() => undefined),
  /** Flow asking the whiteboard to draw, read or erase; answered with "whiteboard-result". */
  whiteboardRequest: (handler: (value: { id: string; action: string; arguments: string }) => void): Promise<UnlistenFn> =>
    isTauri()
      ? listen<{ id: string; action: string; arguments: string }>("whiteboard-request", ({ payload }) => handler(payload))
      : Promise.resolve(() => undefined),
  assistantPlacement: (handler: (value: "above" | "below") => void): Promise<UnlistenFn> =>
    isTauri() ? listen<"above" | "below">("assistant-placement", ({ payload }) => handler(payload)) : Promise.resolve(() => undefined),
};
