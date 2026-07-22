export const DICTATION_STATES = [
  "idle",
  "starting",
  "listening_push_to_talk",
  "listening_hands_free",
  "finalizing_audio",
  "transcribing",
  "cleaning",
  "inserting",
  "completed",
  "cancelled",
  "error",
] as const;

export type DictationState = (typeof DICTATION_STATES)[number];
export type DictationMode = "push_to_talk" | "hands_free" | "call" | "command";
export type InsertionStatus = "inserted" | "copied" | "failed" | "cancelled";
export type PostPasteAction = "none" | "enter" | "tab" | "newline";

export interface DictationSnapshot {
  state: DictationState;
  sessionId?: string;
  mode?: DictationMode;
  startedAt?: string;
  interimTranscript: string;
  error?: AppErrorPayload;
}

export interface AppErrorPayload {
  category: string;
  message: string;
  recoverable: boolean;
}

export interface AudioLevelPayload {
  sessionId: string;
  rms: number;
  peak: number;
  decibels: number;
  bars: number[];
}

export interface ScreenContext {
  application: string | null;
  processName: string | null;
  windowTitle: string | null;
  cursorX: number;
  cursorY: number;
  monitor: {
    left: number;
    top: number;
    width: number;
    height: number;
  };
  screenshotDataUrl: string;
  screenshotWidth: number;
  screenshotHeight: number;
}

export interface TranscriptRecord {
  id: string;
  createdAt: string;
  startedAt: string;
  durationMs: number;
  processingMs: number;
  applicationName?: string;
  processName?: string;
  windowTitle?: string;
  mode: DictationMode;
  rawTranscript: string;
  normalizedTranscript: string;
  cleanedTranscript: string;
  finalTranscript: string;
  transformId?: string;
  provider: string;
  model: string;
  confidence?: number;
  insertionStatus: InsertionStatus;
  postPasteAction: PostPasteAction;
  audioPath?: string;
  isFavorite: boolean;
}

export type DictionaryCategory = "vocabulary" | "replacement" | "protected_identifier";

export interface DictionaryEntry {
  id: string;
  displayTerm: string;
  spokenForms: string[];
  replacement?: string;
  category: DictionaryCategory;
  priority: number;
  caseSensitive: boolean;
  wholeWordOnly: boolean;
  enabled: boolean;
  usageCount: number;
  createdAt: string;
  updatedAt: string;
}

export type DictionaryEntryInput = Omit<Pick<DictionaryEntry, "displayTerm" | "spokenForms" | "replacement" | "category" | "priority" | "caseSensitive" | "wholeWordOnly" | "enabled">, "replacement"> & { replacement: string | null };

export interface AppSettings {
  microphoneName: string | null;
  transcriptionProvider: "deepgram" | "mock";
  transcriptionModel: string;
  language: string;
  cleanupEnabled: boolean;
  cleanupEndpoint: string;
  cleanupModel: string;
  cleanupStyle: "balanced" | "casual" | "developer" | "code_literal";
  autoApplyTransform: string | null;
  pasteDelayMs: number;
  restoreClipboard: boolean;
  pressEnterEnabled: boolean;
  saveHistory: boolean;
  saveAudio: boolean;
  sessionLimitMinutes: number;
  noiseFloorDb: number;
  callModeApplication: string;
  callModeOutputDeviceName: string | null;
  theme: "system" | "light" | "dark";
  buddyStrollEnabled: boolean;
  buddySpeakResponses: boolean;
  assistantEndpoint: string;
  assistantModel: string;
  shortcuts: ShortcutBindings;
  defaultMode: DictationMode;
  autoDetectDeveloperApps: boolean;
  appCleanupStyles: AppCleanupStyle[];
  removeFillerWords: boolean;
  removeFalseStarts: boolean;
  backtrackingEnabled: boolean;
  spokenFormattingEnabled: boolean;
  voiceActionsEnabled: boolean;
  showOverlay: boolean;
  showWaveform: boolean;
  playTones: boolean;
  overlayPosition: OverlayPosition;
  overlayOpacity: number;
  buddyEnabled: boolean;
  buddyShowAtStartup: boolean;
  buddySize: "small" | "medium" | "large";
  buddyAlwaysOnTop: boolean;
  assistantAllowScreenContext: boolean;
  assistantVoice: string | null;
  storeRawTranscript: boolean;
  storeNormalizedTranscript: boolean;
  storeCleanedTranscript: boolean;
  includeTranscriptInLogs: boolean;
  historyRetentionDays: number;
  maxHistoryEntries: number;
  confirmPasteAgain: boolean;
  debugLogging: boolean;
  closeToTray: boolean;
  minimizeToTray: boolean;
  showNotifications: boolean;
  lastPage: string;
}

export type OverlayPosition = "bottom_center" | "bottom_right" | "top_center" | "top_right";
export type CleanupStyle = "balanced" | "casual" | "developer" | "code_literal";

export interface ShortcutBinding {
  /** Any of "ctrl", "alt", "shift", "win". */
  modifiers: string[];
  /** Main key name, or null for a modifier-only gesture such as Ctrl + Win. */
  key: string | null;
}

export interface ShortcutBindings {
  pushToTalk: ShortcutBinding;
  handsFree: ShortcutBinding;
  commandMode: ShortcutBinding;
  cancel: ShortcutBinding;
}

export type ShortcutActionId = keyof ShortcutBindings;

export interface AppCleanupStyle {
  processName: string;
  style: CleanupStyle;
}

export interface CredentialStatus {
  deepgram: boolean;
  cleanup: boolean;
  assistant: boolean;
}

export interface AssistantStateEvent { requestId: string; state: "thinking" | "streaming" | "completed" | "error"; message?: string; }
export interface AssistantDeltaEvent { requestId: string; delta: string; }
export interface AssistantConversationTurn { role: "user" | "assistant"; content: string; }

export interface DashboardStats {
  dailyWords: number;
  dailySessions: number;
  estimatedMinutesSaved: number;
}

export type TransformId = "polish" | "prompt_engineer" | "developer_task" | "bug_report" | "commit_message" | "documentation" | "fix_grammar" | "make_concise" | "turn_into_list";

export interface TransformResponse {
  transformId: string;
  originalText: string;
  transformedText: string;
  provider: string;
}
