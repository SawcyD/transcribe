import { Monitor, Send, X } from "lucide-react";
import { useEffect, useRef, useState } from "react";
import { IconButton } from "@memora/ui";
import { Button } from "../components/common/Button";
import { InfoBar } from "../components/fluent/InfoBar";
import { SettingsRow, SettingsSection } from "../components/fluent/SettingsRow";
import { ToggleSwitch } from "../components/fluent/ToggleSwitch";
import { defaultSettings, events, native } from "../lib/native";
import type { AppSettings, AssistantConversationTurn, ScreenContext } from "../types/models";

/** Voices exposed by the Windows speech synthesis engine. */
function useVoices(): SpeechSynthesisVoice[] {
  const [voices, setVoices] = useState<SpeechSynthesisVoice[]>([]);
  useEffect(() => {
    const read = () => setVoices(window.speechSynthesis?.getVoices() ?? []);
    read();
    window.speechSynthesis?.addEventListener("voiceschanged", read);
    return () => window.speechSynthesis?.removeEventListener("voiceschanged", read);
  }, []);
  return voices;
}

export function AssistantPage() {
  const [settings, setSettings] = useState<AppSettings>(defaultSettings);
  const [turns, setTurns] = useState<AssistantConversationTurn[]>([]);
  const [prompt, setPrompt] = useState("");
  const [streaming, setStreaming] = useState("");
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [context, setContext] = useState<ScreenContext | null>(null);
  const conversation = useRef<HTMLDivElement>(null);
  const voices = useVoices();

  useEffect(() => {
    void native.settings().then(setSettings);
  }, []);

  useEffect(() => {
    const deltas = events.assistantDelta((event) => setStreaming((current) => current + event.delta));
    return () => {
      void deltas.then((unlisten) => unlisten());
    };
  }, []);

  useEffect(() => {
    conversation.current?.scrollTo({ top: conversation.current.scrollHeight });
  }, [turns, streaming]);

  const update = async <K extends keyof AppSettings>(key: K, value: AppSettings[K]) => {
    const next = { ...settings, [key]: value };
    setSettings(next);
    await native.saveSettings(next);
  };

  const attachContext = async () => {
    try {
      setContext(await native.captureScreenContext());
      setError(null);
    } catch (cause) {
      setError(cause instanceof Error ? cause.message : "Could not capture the screen.");
    }
  };

  const send = async () => {
    const text = prompt.trim();
    if (!text || busy) return;
    setBusy(true);
    setError(null);
    setStreaming("");
    const history = [...turns];
    setTurns([...history, { role: "user", content: text }]);
    setPrompt("");
    try {
      const answer = await native.askAssistant(text, context, history);
      setTurns((current) => [...current, { role: "assistant", content: answer }]);
      setStreaming("");
      // Screen context is deliberately single-use, so a stale capture is never
      // silently attached to a later question.
      setContext(null);
      if (settings.buddySpeakResponses && window.speechSynthesis) {
        const utterance = new SpeechSynthesisUtterance(answer);
        const voice = voices.find((entry) => entry.name === settings.assistantVoice);
        if (voice) utterance.voice = voice;
        window.speechSynthesis.speak(utterance);
      }
    } catch (cause) {
      setError(cause instanceof Error ? cause.message : "The assistant could not answer.");
      setStreaming("");
    } finally {
      setBusy(false);
    }
  };

  return (
    <div className="page">
      <header className="page-header">
        <h1>Assistant</h1>
        <p className="page-header__meta">
          <span>
            The assistant can answer questions using your message and optional screen context. It cannot click, type, or
            control your desktop.
          </span>
        </p>
      </header>

      {error && <InfoBar severity="error" title="Assistant" message={error} />}

      <div className="chat" ref={conversation}>
        {turns.length === 0 && !streaming ? (
          <div className="empty-state">
            <strong>Ask VoiceFlow anything</strong>
            <p>Type below, or hold your Command Mode shortcut and speak.</p>
          </div>
        ) : (
          <>
            {turns.map((turn, index) => (
              <div key={index} className={`chat-turn chat-turn--${turn.role}`}>
                <span className="chat-turn__role">{turn.role === "user" ? "You" : "Assistant"}</span>
                <p>{turn.content}</p>
              </div>
            ))}
            {streaming && (
              <div className="chat-turn chat-turn--assistant">
                <span className="chat-turn__role">Assistant</span>
                <p>{streaming}</p>
              </div>
            )}
          </>
        )}
      </div>

      {context && (
        <div className="context-chip">
          <Monitor size={15} aria-hidden="true" />
          <span>
            Screen context attached
            <small>Captured from {context.application ?? "the active window"}</small>
          </span>
          <IconButton label="Remove screen context" variant="subtle" onClick={() => setContext(null)}><X size={15} /></IconButton>
        </div>
      )}

      <div className="composer">
        <textarea
          value={prompt}
          aria-label="Ask VoiceFlow"
          placeholder="Ask VoiceFlow…"
          onChange={(event) => setPrompt(event.target.value)}
          onKeyDown={(event) => {
            if (event.key === "Enter" && (event.ctrlKey || event.metaKey)) {
              event.preventDefault();
              void send();
            }
          }}
        />
        <div className="composer__actions">
          <Button
            variant="secondary"
            icon={<Monitor size={15} />}
            disabled={!settings.assistantAllowScreenContext || busy}
            onClick={() => void attachContext()}
          >
            Attach screen context
          </Button>
          <small className="composer__hint">Ctrl + Enter to send</small>
          <Button variant="primary" icon={<Send size={15} />} disabled={!prompt.trim() || busy} onClick={() => void send()}>
            {busy ? "Sending…" : "Send"}
          </Button>
        </div>
      </div>

      <SettingsSection title="Assistant options">
        <SettingsRow
          label="Allow screen context"
          description="When off, the assistant can never capture your screen, including from Buddy."
          action={
            <ToggleSwitch
              label="Allow screen context"
              checked={settings.assistantAllowScreenContext}
              onChange={(value) => void update("assistantAllowScreenContext", value)}
            />
          }
        />
        <SettingsRow
          label="Read responses aloud"
          action={
            <ToggleSwitch
              label="Read responses aloud"
              checked={settings.buddySpeakResponses}
              onChange={(value) => void update("buddySpeakResponses", value)}
            />
          }
        />
        <SettingsRow
          label="Voice"
          action={
            <select
              className="settings-combo"
              aria-label="Assistant voice"
              value={settings.assistantVoice ?? ""}
              onChange={(event) => void update("assistantVoice", event.target.value || null)}
            >
              <option value="">System default</option>
              {voices.map((voice) => (
                <option key={voice.name} value={voice.name}>
                  {voice.name}
                </option>
              ))}
            </select>
          }
        />
      </SettingsSection>

      <SettingsSection title="Buddy">
        <SettingsRow
          label="Enable Buddy"
          description="The desktop companion that mirrors dictation state and opens the assistant."
          action={
            <ToggleSwitch
              label="Enable Buddy"
              checked={settings.buddyEnabled}
              onChange={async (value) => {
                await update("buddyEnabled", value);
                await native.applyBuddySettings();
              }}
            />
          }
        />
        <SettingsRow
          label="Show Buddy when VoiceFlow starts"
          action={
            <ToggleSwitch
              label="Show Buddy when VoiceFlow starts"
              checked={settings.buddyShowAtStartup}
              onChange={(value) => void update("buddyShowAtStartup", value)}
            />
          }
        />
        <SettingsRow
          label="Buddy size"
          action={
            <select
              className="settings-combo"
              aria-label="Buddy size"
              value={settings.buddySize}
              onChange={async (event) => {
                await update("buddySize", event.target.value as AppSettings["buddySize"]);
                await native.applyBuddySettings();
              }}
            >
              <option value="small">Small</option>
              <option value="medium">Medium</option>
              <option value="large">Large</option>
            </select>
          }
        />
        <SettingsRow
          label="Keep above other windows"
          action={
            <ToggleSwitch
              label="Keep above other windows"
              checked={settings.buddyAlwaysOnTop}
              onChange={async (value) => {
                await update("buddyAlwaysOnTop", value);
                await native.applyBuddySettings();
              }}
            />
          }
        />
      </SettingsSection>
    </div>
  );
}
