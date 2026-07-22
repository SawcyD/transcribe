import { Headphones, MessageSquare, Mic, Play } from "lucide-react";
import { useEffect, useState } from "react";
import { useNavigate } from "react-router-dom";
import { InfoRow, ProgressBar, SectionHeader } from "@memora/ui";
import { useAppStore } from "../app/useAppStore";
import { Button } from "../components/common/Button";
import { InfoBar } from "../components/fluent/InfoBar";
import { StatusIndicator } from "../components/fluent/StatusIndicator";
import { native } from "../lib/native";
import type { AppSettings, CredentialStatus, DictationSnapshot, TranscriptRecord } from "../types/models";

const SHORTCUTS = [
  { keys: ["Ctrl", "Win"], label: "Push to talk", description: "Hold while speaking, release to insert." },
  { keys: ["Ctrl", "Win", "Space"], label: "Hands-free dictation", description: "Press to start, press again to finish." },
  { keys: ["Ctrl", "Alt", "B"], label: "Command Mode", description: "Speak a request to the assistant." },
  { keys: ["Esc"], label: "Cancel", description: "Discard the active dictation." },
];

/** Human-readable primary status line, derived from the dictation state machine. */
function statusHeadline(snapshot: DictationSnapshot, configured: boolean): string {
  switch (snapshot.state) {
    case "starting":
      return "Starting microphone…";
    case "listening_push_to_talk":
    case "listening_hands_free":
      return "Listening";
    case "finalizing_audio":
      return "Finishing transcription…";
    case "transcribing":
      return "Processing transcription";
    case "cleaning":
      return "Cleaning text…";
    case "inserting":
      return "Inserting…";
    case "completed":
      return "Inserted successfully";
    case "cancelled":
      return "Dictation cancelled";
    case "error":
      return snapshot.error?.message ?? "Something went wrong";
    default:
      return configured ? "Ready" : "Transcription is not configured";
  }
}

function elapsedLabel(startedAt: string | null | undefined): string | null {
  if (!startedAt) return null;
  const seconds = Math.max(0, Math.floor((Date.now() - new Date(startedAt).getTime()) / 1000));
  return `${String(Math.floor(seconds / 60)).padStart(2, "0")}:${String(seconds % 60).padStart(2, "0")}`;
}

export function HomePage() {
  const navigate = useNavigate();
  const { dictation } = useAppStore();
  const [settings, setSettings] = useState<AppSettings | null>(null);
  const [credentials, setCredentials] = useState<CredentialStatus>({ deepgram: false, cleanup: false, assistant: false });
  const [recent, setRecent] = useState<TranscriptRecord[]>([]);
  const [, forceTick] = useState(0);

  useEffect(() => {
    void Promise.all([native.settings(), native.credentialStatus(), native.history()]).then(
      ([nextSettings, nextCredentials, transcripts]) => {
        setSettings(nextSettings);
        setCredentials(nextCredentials);
        setRecent(transcripts.slice(0, 5));
      },
    );
  }, [dictation.state]);

  const listening = dictation.state === "listening_push_to_talk" || dictation.state === "listening_hands_free";

  // Drive the recording timer without re-rendering the page when idle.
  useEffect(() => {
    if (!listening) return;
    const id = window.setInterval(() => forceTick((tick) => tick + 1), 1000);
    return () => window.clearInterval(id);
  }, [listening]);

  const configured = credentials.deepgram;
  const elapsed = listening ? elapsedLabel(dictation.startedAt) : null;

  return (
    <div className="page">
      <header className="page-header">
        <h1>VoiceFlow</h1>
        <p className="page-header__meta">
          <StatusIndicator state={dictation.state} label={configured ? undefined : "Not configured"} tone={configured ? undefined : "error"} />
          <span>Microphone: {settings?.microphoneName ?? "System default"}</span>
          <span>
            Transcription: {settings?.transcriptionProvider === "deepgram" ? "Deepgram" : settings?.transcriptionProvider}{" "}
            {settings?.transcriptionModel}
          </span>
        </p>
      </header>

      {!configured && (
        <InfoBar
          severity="warning"
          title="Transcription is not configured"
          message="Add your Deepgram credential to begin using dictation. It is stored in Windows Credential Manager."
          action={
            <Button variant="primary" onClick={() => navigate("/settings")}>
              Configure provider
            </Button>
          }
        />
      )}

      <section className="status-panel" aria-label="Dictation status">
        <div className="status-panel__heading">
          <div>
            <SectionHeader>Dictation</SectionHeader>
            <p className="status-panel__headline">{statusHeadline(dictation, configured)}</p>
          </div>
          <Button
            variant={listening ? "danger" : "primary"}
            disabled={!configured && !listening}
            icon={listening ? <Mic size={16} /> : <Play size={16} />}
            onClick={() => void (listening ? native.finish() : native.start("hands_free"))}
          >
            {listening ? "Finish dictation" : "Start dictation"}
          </Button>
        </div>
        {elapsed && <p className="status-panel__timer">{elapsed}</p>}
        <p className="status-panel__detail">
          {dictation.interimTranscript ||
            (listening ? `Input: ${settings?.microphoneName ?? "System default"}` : "Hold Ctrl + Win to dictate")}
        </p>
        {listening && <ProgressBar indeterminate label="Receiving microphone audio" />}
        <div className="voiceflow-status-rows">
          <InfoRow label="Mode" value={dictation.mode?.replaceAll("_", " ") ?? settings?.defaultMode?.replaceAll("_", " ") ?? "Push to talk"} />
          <InfoRow label="Cleanup" value={settings?.cleanupEnabled ? settings.cleanupStyle : "Off"} />
        </div>
      </section>

      <section className="settings-group" aria-label="Shortcuts">
        <h2 className="settings-group__title">Shortcuts</h2>
        <div className="settings-group__rows">
          {SHORTCUTS.map((shortcut) => (
            <div className="settings-row" key={shortcut.label}>
              <span className="settings-row__text">
                <strong>{shortcut.label}</strong>
                <small>{shortcut.description}</small>
              </span>
              <span className="settings-row__action">
                <span className="shortcut-chip">
                  {shortcut.keys.map((key) => (
                    <kbd key={key}>{key}</kbd>
                  ))}
                </span>
              </span>
            </div>
          ))}
        </div>
      </section>

      <div className="command-row command-row--secondary">
        <Button variant="secondary" disabled={!configured || listening} icon={<MessageSquare size={16} />} onClick={() => void native.start("command")}>
          Open Command Mode
        </Button>
        <Button variant="secondary" disabled={!configured || listening} icon={<Headphones size={16} />} onClick={() => void native.start("call")}>
          Call Mode
        </Button>
      </div>

      <section className="settings-group" aria-label="Recent activity">
        <h2 className="settings-group__title">Recent activity</h2>
        {recent.length === 0 ? (
          <div className="empty-state">
            <strong>No dictations yet</strong>
            <p>Hold Ctrl + Win and start speaking.</p>
          </div>
        ) : (
          <div className="settings-group__rows">
            {recent.map((record) => (
              <button
                type="button"
                key={record.id}
                className="settings-row settings-row--interactive activity-row"
                onClick={() => navigate("/history", { state: { transcriptId: record.id } })}
              >
                <time>{new Date(record.createdAt).toLocaleTimeString([], { hour: "numeric", minute: "2-digit" })}</time>
                <span className="activity-row__app">{record.applicationName ?? "Unknown app"}</span>
                <span className="activity-row__words">
                  {record.finalTranscript.trim().split(/\s+/).filter(Boolean).length} words
                </span>
              </button>
            ))}
          </div>
        )}
      </section>
    </div>
  );
}
