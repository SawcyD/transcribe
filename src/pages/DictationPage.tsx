import { AudioLines, Eye, Hand, Headphones, Mic, Plus, Trash2 } from "lucide-react";
import { useAutoSave } from "../app/useAutoSave";
import { SaveIndicator } from "../components/common/SaveIndicator";
import { useEffect, useMemo, useState } from "react";
import { ComboBox, NumberBox } from "@memora/ui";
import { Button } from "../components/common/Button";
import { InfoBar } from "../components/fluent/InfoBar";
import { SettingsRow, SettingsSection } from "../components/fluent/SettingsRow";
import { ShortcutRecorder } from "../components/fluent/ShortcutRecorder";
import { ToggleSwitch } from "../components/fluent/ToggleSwitch";
import { defaultSettings, events, native } from "../lib/native";
import { bindingKeys } from "../lib/shortcutKeys";
import type {
  AppSettings,
  CleanupStyle,
  DictationMode,
  ShortcutActionId,
} from "../types/models";
import { validateSettings } from "./settingsValidation";

/** Modes the Home "Start dictation" button and the tray can start. */
const MODES: Array<{ id: DictationMode; label: string }> = [
  { id: "push_to_talk", label: "Dictate" },
  { id: "hands_free", label: "Dictate hands-free" },
  { id: "call", label: "Call notes" },
];

const MODE_CARDS: Array<{
  title: string;
  icon: typeof Mic;
  shortcut?: ShortcutActionId;
  startHint?: string;
  how: string;
  result: string;
  assistant?: boolean;
}> = [
  { title: "Dictate", icon: Mic, shortcut: "pushToTalk", how: "Hold the keys, speak, let go.", result: "Typed where your cursor is" },
  { title: "Dictate hands-free", icon: Hand, shortcut: "handsFree", how: "Press once, talk as long as you like, press again (or say “finish dictation”).", result: "Typed where your cursor is" },
  { title: "Ask about your screen", icon: Eye, shortcut: "commandMode", how: "Hold and ask a question. It looks at the window you're in.", result: "Answer pops up above the dock", assistant: true },
  { title: "Talk to the assistant", icon: AudioLines, shortcut: "voiceAgent", how: "Press to start a spoken back-and-forth; press again to end.", result: "It talks back; saved in Assistant", assistant: true },
  { title: "Call notes", icon: Headphones, startHint: "Start from Home or the tray", how: "Records your computer's audio (Stereo Mix or a virtual cable).", result: "Notes copied to the clipboard" },
];

const STYLES: Array<{ id: CleanupStyle; label: string; description: string }> = [
  { id: "balanced", label: "Balanced", description: "Corrects grammar and structure while preserving your wording." },
  { id: "casual", label: "Casual", description: "Keeps informal phrasing and contractions intact." },
  { id: "developer", label: "Developer", description: "Preserves technical terms and formats developer-focused speech clearly." },
  { id: "code_literal", label: "Code literal", description: "Avoids rewriting code, identifiers, commands, and syntax." },
];

const SHORTCUT_LABELS: Array<{ id: ShortcutActionId; label: string; description: string }> = [
  { id: "pushToTalk", label: "Push to talk", description: "Hold to dictate; release to insert." },
  { id: "handsFree", label: "Hands-free dictation", description: "Press to start, press again to finish." },
  { id: "commandMode", label: "Command Mode", description: "Speak a request to the assistant." },
  { id: "cancel", label: "Cancel", description: "Discard the active dictation." },
  { id: "voiceAgent", label: "Voice agent", description: "Start a spoken conversation with the assistant; press again to end it." },
];

export function DictationPage() {
  const [settings, setSettings] = useState<AppSettings>(defaultSettings);
  const [microphones, setMicrophones] = useState<string[]>([]);
  const [level, setLevel] = useState(0);
  const [testing, setTesting] = useState(false);
  const [loaded, setLoaded] = useState(false);
  const [newMapping, setNewMapping] = useState({ processName: "", style: "developer" as CleanupStyle });

  const errors = useMemo(() => validateSettings(settings), [settings]);
  const valid = Object.keys(errors).length === 0;

  useEffect(() => {
    void Promise.all([native.settings(), native.microphones()]).then(([stored, devices]) => {
      setSettings(stored);
      setMicrophones(devices);
      setLoaded(true);
    });
  }, []);

  // The backend already publishes levels; the meter reuses that stream rather
  // than opening its own capture.
  useEffect(() => {
    if (!testing) return;
    const pending = events.audio((payload) => setLevel(payload.rms));
    return () => {
      void pending.then((unlisten) => unlisten());
    };
  }, [testing]);

  const update = <K extends keyof AppSettings>(key: K, value: AppSettings[K]) =>
    setSettings((current) => ({ ...current, [key]: value }));

  const saveState = useAutoSave(settings, {
    ready: loaded,
    valid,
    persist: async (next) => {
      const stored = await native.settings();
      await native.saveSettings({ ...next, lastPage: stored.lastPage });
    },
  });

  const stereoMixAvailable = microphones.some((name) => /stereo mix|virtual|cable|loopback/i.test(name));

  return (
    <div className="page">
      <header className="page-header">
        <h1>Dictation</h1>
        <p className="page-header__meta">
          <span>Recording modes, input devices, cleanup behaviour, and insertion. Changes save automatically.</span>
        </p>
      </header>

      {!valid && <InfoBar severity="error" title="Some settings are invalid" message={Object.values(errors)[0]} />}

      <SettingsSection title="How VoiceFlow listens" description="Five ways to use your voice. Change any shortcut below.">
        <div className="mode-cards">
          {MODE_CARDS.map((card) => {
            const Icon = card.icon;
            const keys = card.shortcut ? bindingKeys(settings.shortcuts[card.shortcut]) : [];
            return (
              <div key={card.title} className={`mode-card${card.assistant ? " mode-card--assistant" : ""}`}>
                <span className="mode-card__icon" aria-hidden="true">
                  <Icon size={16} />
                </span>
                <div className="mode-card__body">
                  <span className="mode-card__title">
                    <strong>{card.title}</strong>
                    {keys.length > 0 ? (
                      <span className="shortcut-chip">{keys.map((key) => <kbd key={key}>{key}</kbd>)}</span>
                    ) : (
                      <small className="mode-card__start">{card.startHint}</small>
                    )}
                  </span>
                  <span className="mode-card__how">{card.how}</span>
                  <span className="mode-card__result">→ {card.result}</span>
                </div>
              </div>
            );
          })}
        </div>
        <SettingsRow
          label="Default mode"
          description="What the Home page's Start button and the tray icon do."
          action={
            <ComboBox label="Default mode" value={settings.defaultMode} onChange={(value) => update("defaultMode", value)} options={MODES.map((entry) => ({ value: entry.id, label: entry.label }))} />
          }
        />
      </SettingsSection>

      <SettingsSection title="Shortcuts" description="New shortcuts take effect immediately.">
        {SHORTCUT_LABELS.map((entry) => (
          <SettingsRow
            key={entry.id}
            label={entry.label}
            description={entry.description}
            action={
              <ShortcutRecorder
                label={entry.label}
                value={settings.shortcuts[entry.id]}
                onChange={(binding) =>
                  update("shortcuts", { ...settings.shortcuts, [entry.id]: binding })
                }
                onReset={() =>
                  update("shortcuts", {
                    ...settings.shortcuts,
                    [entry.id]: defaultSettings.shortcuts[entry.id],
                  })
                }
              />
            }
          />
        ))}
      </SettingsSection>

      <SettingsSection title="Input device">
        <SettingsRow
          label="Microphone"
          action={
            <ComboBox label="Microphone" value={settings.microphoneName ?? ""} onChange={(value) => update("microphoneName", value || null)} options={[{ value: "", label: "System default" }, ...microphones.map((name) => ({ value: name, label: name }))]} />
          }
        />
        <SettingsRow
          label="Test microphone"
          description="Starts a hands-free session so you can watch the input level, then finishes it."
          action={
            <span className="level-control">
              <span className="level-meter" role="meter" aria-label="Input level" aria-valuenow={Math.round(level * 100)} aria-valuemin={0} aria-valuemax={100}>
                <i style={{ width: `${Math.min(100, Math.round(level * 140))}%` }} />
              </span>
              <Button
                variant="secondary"
                onClick={async () => {
                  if (testing) {
                    await native.finish();
                    setTesting(false);
                    setLevel(0);
                  } else {
                    await native.start("hands_free");
                    setTesting(true);
                  }
                }}
              >
                {testing ? "Stop test" : "Test"}
              </Button>
            </span>
          }
        />
        <SettingsRow
          label="Noise floor"
          description="Audio quieter than this is treated as silence."
          action={
            <NumberBox label="Noise floor in decibels" value={settings.noiseFloorDb} min={-90} max={-10} suffix="dB" onChange={(value) => update("noiseFloorDb", value)} />
          }
        />
        <SettingsRow
          label="Session limit"
          description="Long sessions finish automatically at this limit."
          action={
            <NumberBox label="Session limit in minutes" value={settings.sessionLimitMinutes} min={1} max={120} suffix="min" onChange={(value) => update("sessionLimitMinutes", value)} />
          }
        />
      </SettingsSection>

      <SettingsSection title="Call Mode">
        {!stereoMixAvailable && (
          <InfoBar
            severity="warning"
            title="Stereo Mix is not available on this device"
            message="Enable Stereo Mix in Windows sound settings, or install a virtual audio cable and route the call application to it."
          />
        )}
        <SettingsRow
          label="Call application"
          description="Used to label the saved call notes."
          action={
            <input
              className="settings-input"
              aria-label="Call application"
              value={settings.callModeApplication}
              onChange={(event) => update("callModeApplication", event.target.value)}
            />
          }
        />
        <SettingsRow
          label="Call audio source"
          action={
            <ComboBox label="Call audio source" value={settings.callModeOutputDeviceName ?? ""} onChange={(value) => update("callModeOutputDeviceName", value || null)} options={[{ value: "", label: "Use selected microphone" }, ...microphones.map((name) => ({ value: name, label: name }))]} />
          }
        />
      </SettingsSection>

      <SettingsSection title="Transcription">
        <SettingsRow label="Provider" action={<span className="settings-row__value">Deepgram</span>} />
        <SettingsRow
          label="Model"
          action={
            <input
              className="settings-input"
              aria-label="Transcription model"
              value={settings.transcriptionModel}
              onChange={(event) => update("transcriptionModel", event.target.value)}
            />
          }
        />
        <SettingsRow
          label="Language"
          action={
            <select
              className="settings-combo"
              aria-label="Language"
              value={settings.language}
              onChange={(event) => update("language", event.target.value)}
            >
              <option value="en-US">English (United States)</option>
              <option value="en-GB">English (United Kingdom)</option>
              <option value="multi">Multilingual</option>
            </select>
          }
        />
        <SettingsRow label="Streaming transcription" action={<span className="settings-row__value">On</span>} />
        <SettingsRow
          label="Keep connection warm"
          description="Keeps a Deepgram connection ready between dictations so recording starts without a handshake. No audio is sent while idle; closes after 5 minutes unused."
          action={
            <ToggleSwitch
              label="Keep connection warm"
              checked={settings.keepTranscriptionWarm}
              onChange={(value) => update("keepTranscriptionWarm", value)}
            />
          }
        />
      </SettingsSection>

      <SettingsSection title="Cleanup">
        <SettingsRow
          label="AI cleanup"
          description="Send normalized text to the configured cleanup provider."
          action={<ToggleSwitch label="AI cleanup" checked={settings.cleanupEnabled} onChange={(value) => update("cleanupEnabled", value)} />}
        />
        <SettingsRow
          label="Skip AI for short dictations"
          description="Shorter dictations insert faster using local cleanup only. Spoken code (e.g. “snake case”) still uses AI. 0 = always use AI."
          action={
            <span className="numeric-field">
              <input
                type="number"
                aria-label="Minimum words for AI cleanup"
                min={0}
                max={50}
                disabled={!settings.cleanupEnabled}
                value={settings.cleanupMinWords}
                onChange={(event) => update("cleanupMinWords", Number(event.target.value))}
              />
              <small>words</small>
            </span>
          }
        />
        <SettingsRow
          label="Cleanup style"
          description={STYLES.find((entry) => entry.id === settings.cleanupStyle)?.description}
          action={
            <select
              className="settings-combo"
              aria-label="Cleanup style"
              value={settings.cleanupStyle}
              onChange={(event) => update("cleanupStyle", event.target.value as CleanupStyle)}
            >
              {STYLES.map((entry) => (
                <option key={entry.id} value={entry.id}>
                  {entry.label}
                </option>
              ))}
            </select>
          }
        />
        <SettingsRow
          label="Remove filler words"
          description={'Drops "um", "uh", "you know", and similar.'}
          action={<ToggleSwitch label="Remove filler words" checked={settings.removeFillerWords} onChange={(value) => update("removeFillerWords", value)} />}
        />
        <SettingsRow
          label="Remove false starts"
          description="Collapses repeated phrases caused by restarting a sentence."
          action={<ToggleSwitch label="Remove false starts" checked={settings.removeFalseStarts} onChange={(value) => update("removeFalseStarts", value)} />}
        />
        <SettingsRow
          label="Enable backtracking"
          description={'Applies spoken corrections such as "scratch that" and "replace X with Y".'}
          action={<ToggleSwitch label="Enable backtracking" checked={settings.backtrackingEnabled} onChange={(value) => update("backtrackingEnabled", value)} />}
        />
        <SettingsRow
          label="Spoken formatting"
          description={'Turns "new line", "new paragraph", and list phrases into real structure.'}
          action={<ToggleSwitch label="Spoken formatting" checked={settings.spokenFormattingEnabled} onChange={(value) => update("spokenFormattingEnabled", value)} />}
        />
        <SettingsRow
          label="Voice actions"
          description={'Recognises trailing commands such as "press tab" and "finish dictation".'}
          action={<ToggleSwitch label="Voice actions" checked={settings.voiceActionsEnabled} onChange={(value) => update("voiceActionsEnabled", value)} />}
        />
        <SettingsRow
          label='Allow "press enter"'
          description="Off by default, because a stray match would submit the focused form."
          action={<ToggleSwitch label="Allow press enter" checked={settings.pressEnterEnabled} onChange={(value) => update("pressEnterEnabled", value)} />}
        />
      </SettingsSection>

      <SettingsSection
        title="App-aware behaviour"
        description="An explicit mapping always wins over the default cleanup style."
      >
        <SettingsRow
          label="Automatically select cleanup style based on active app"
          action={
            <ToggleSwitch
              label="Automatically select cleanup style based on active app"
              checked={settings.autoDetectDeveloperApps}
              onChange={(value) => update("autoDetectDeveloperApps", value)}
            />
          }
        />
        {settings.appCleanupStyles.map((mapping, index) => (
          <div className="settings-row" key={`${mapping.processName}-${index}`}>
            <span className="settings-row__text">
              <strong>{mapping.processName}</strong>
            </span>
            <span className="settings-row__action">
              <select
                className="settings-combo"
                aria-label={`Cleanup style for ${mapping.processName}`}
                value={mapping.style}
                onChange={(event) => {
                  const next = [...settings.appCleanupStyles];
                  next[index] = { ...mapping, style: event.target.value as CleanupStyle };
                  update("appCleanupStyles", next);
                }}
              >
                {STYLES.map((entry) => (
                  <option key={entry.id} value={entry.id}>
                    {entry.label}
                  </option>
                ))}
              </select>
              <button
                type="button"
                className="icon-button"
                aria-label={`Remove mapping for ${mapping.processName}`}
                onClick={() =>
                  update(
                    "appCleanupStyles",
                    settings.appCleanupStyles.filter((_, position) => position !== index),
                  )
                }
              >
                <Trash2 size={15} />
              </button>
            </span>
          </div>
        ))}
        <div className="settings-row">
          <span className="settings-row__text">
            <input
              className="settings-input"
              aria-label="New application process name"
              placeholder="application.exe"
              value={newMapping.processName}
              onChange={(event) => setNewMapping({ ...newMapping, processName: event.target.value })}
            />
          </span>
          <span className="settings-row__action">
            <select
              className="settings-combo"
              aria-label="Cleanup style for the new application"
              value={newMapping.style}
              onChange={(event) => setNewMapping({ ...newMapping, style: event.target.value as CleanupStyle })}
            >
              {STYLES.map((entry) => (
                <option key={entry.id} value={entry.id}>
                  {entry.label}
                </option>
              ))}
            </select>
            <Button
              variant="secondary"
              icon={<Plus size={15} />}
              disabled={!newMapping.processName.trim()}
              onClick={() => {
                update("appCleanupStyles", [
                  ...settings.appCleanupStyles,
                  { processName: newMapping.processName.trim().toLowerCase(), style: newMapping.style },
                ]);
                setNewMapping({ processName: "", style: "developer" });
              }}
            >
              Add
            </Button>
          </span>
        </div>
      </SettingsSection>

      <SettingsSection title="Insertion">
        <SettingsRow
          label="Paste delay"
          description="Time allowed for the target window to regain focus before pasting."
          action={
            <span className="numeric-field">
              <input
                type="number"
                aria-label="Paste delay in milliseconds"
                min={40}
                max={2000}
                value={settings.pasteDelayMs}
                onChange={(event) => update("pasteDelayMs", Number(event.target.value))}
              />
              <small>ms</small>
            </span>
          }
        />
        <SettingsRow
          label="Restore clipboard after paste"
          description="Best-effort: some applications read pasted text asynchronously."
          action={<ToggleSwitch label="Restore clipboard after paste" checked={settings.restoreClipboard} onChange={(value) => update("restoreClipboard", value)} />}
        />
        <SettingsRow
          label="Save recordings"
          description="Off by default. Audio is otherwise held in memory only for error recovery."
          action={<ToggleSwitch label="Save recordings" checked={settings.saveAudio} onChange={(value) => update("saveAudio", value)} />}
        />
      </SettingsSection>

      <SaveIndicator state={saveState} />
    </div>
  );
}
