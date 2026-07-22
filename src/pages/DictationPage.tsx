import { Plus, Trash2 } from "lucide-react";
import { useEffect, useMemo, useState } from "react";
import { ComboBox, NumberBox } from "@memora/ui";
import { Button } from "../components/common/Button";
import { InfoBar } from "../components/fluent/InfoBar";
import { SettingsRow, SettingsSection } from "../components/fluent/SettingsRow";
import { ShortcutRecorder } from "../components/fluent/ShortcutRecorder";
import { ToggleSwitch } from "../components/fluent/ToggleSwitch";
import { defaultSettings, events, native } from "../lib/native";
import type {
  AppSettings,
  CleanupStyle,
  DictationMode,
  ShortcutActionId,
} from "../types/models";
import { validateSettings } from "./settingsValidation";

const MODES: Array<{ id: DictationMode; label: string; explanation: string }> = [
  {
    id: "push_to_talk",
    label: "Push to talk",
    explanation: "Hold the shortcut while speaking. Release the keys to process and insert the transcript.",
  },
  {
    id: "hands_free",
    label: "Hands-free",
    explanation: "Press the shortcut to start. Press it again to finish.",
  },
  {
    id: "command",
    label: "Command Mode",
    explanation: "Speak a request to the assistant. The result is returned in the Assistant panel.",
  },
  {
    id: "call",
    label: "Call Mode",
    explanation:
      "Records from Stereo Mix or a virtual audio cable. Results are formatted as call notes and copied to the clipboard.",
  },
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
];

export function DictationPage() {
  const [settings, setSettings] = useState<AppSettings>(defaultSettings);
  const [microphones, setMicrophones] = useState<string[]>([]);
  const [mode, setMode] = useState<DictationMode>("push_to_talk");
  const [level, setLevel] = useState(0);
  const [testing, setTesting] = useState(false);
  const [saved, setSaved] = useState(false);
  const [newMapping, setNewMapping] = useState({ processName: "", style: "developer" as CleanupStyle });

  const errors = useMemo(() => validateSettings(settings), [settings]);
  const valid = Object.keys(errors).length === 0;

  useEffect(() => {
    void Promise.all([native.settings(), native.microphones()]).then(([stored, devices]) => {
      setSettings(stored);
      setMode(stored.defaultMode);
      setMicrophones(devices);
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

  const save = async () => {
    if (!valid) return;
    const stored = await native.saveSettings(settings);
    setSettings(stored);
    setSaved(true);
    window.setTimeout(() => setSaved(false), 2000);
  };

  const stereoMixAvailable = microphones.some((name) => /stereo mix|virtual|cable|loopback/i.test(name));
  const activeMode = MODES.find((entry) => entry.id === mode) ?? MODES[0];

  return (
    <div className="page">
      <header className="page-header">
        <h1>Dictation</h1>
        <p className="page-header__meta">
          <span>Recording modes, input devices, cleanup behaviour, and insertion.</span>
        </p>
      </header>

      {!valid && <InfoBar severity="error" title="Some settings are invalid" message={Object.values(errors)[0]} />}

      <SettingsSection title="Mode">
        <div className="segmented" role="tablist" aria-label="Dictation mode">
          {MODES.map((entry) => (
            <button
              key={entry.id}
              type="button"
              role="tab"
              aria-selected={mode === entry.id}
              className={`segmented__item${mode === entry.id ? " segmented__item--selected" : ""}`}
              onClick={() => setMode(entry.id)}
            >
              {entry.label}
            </button>
          ))}
        </div>
        <p className="settings-group__description mode-explanation">{activeMode.explanation}</p>
        <SettingsRow
          label="Default mode"
          description="Used by the Home page and tray when you start dictation without a shortcut."
          action={
            <ComboBox label="Default mode" value={settings.defaultMode} onChange={(value) => update("defaultMode", value)} options={MODES.map((entry) => ({ value: entry.id, label: entry.label }))} />
          }
        />
      </SettingsSection>

      <SettingsSection title="Shortcuts" description="Changes take effect as soon as you save.">
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
      </SettingsSection>

      <SettingsSection title="Cleanup">
        <SettingsRow
          label="AI cleanup"
          description="Send normalized text to the configured cleanup provider."
          action={<ToggleSwitch label="AI cleanup" checked={settings.cleanupEnabled} onChange={(value) => update("cleanupEnabled", value)} />}
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

      <div className="command-row">
        <Button variant="primary" disabled={!valid} onClick={() => void save()}>
          {saved ? "Saved" : "Save changes"}
        </Button>
      </div>
    </div>
  );
}
