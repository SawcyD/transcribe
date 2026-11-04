import { Check, FileText, X } from "lucide-react";
import { useEffect, useRef, useState } from "react";
import { useAppStore } from "../app/useAppStore";
import { native } from "../lib/native";
import { AudioVisualizer } from "./AudioVisualizer";
import { captionTail, overlayPresentation } from "./OverlayState";
import { useTones } from "./useTones";
import type { AppSettings } from "../types/models";
import { defaultSettings } from "../lib/native";

function elapsed(startedAt: string | null | undefined): string {
  if (!startedAt) return "0:00";
  const seconds = Math.max(0, Math.floor((Date.now() - new Date(startedAt).getTime()) / 1000));
  return `${Math.floor(seconds / 60)}:${String(seconds % 60).padStart(2, "0")}`;
}

/** Visualizer bars are 0–1 above the noise floor; below this is silence. */
const SOUND_THRESHOLD = 0.08;
const SILENCE_HINT_MS = 3_000;

export function RecordingOverlay() {
  const { dictation, audio } = useAppStore();
  const [settings, setSettings] = useState<AppSettings>(defaultSettings);
  const [, tick] = useState(0);

  // Settings only change between sessions, so reload when a session starts.
  const sessionId = dictation.sessionId;
  useEffect(() => {
    void native.settings().then(setSettings);
  }, [sessionId]);

  const presentation = overlayPresentation(dictation.state, null, dictation.error, dictation.interimTranscript);
  useTones(dictation.state, settings.playTones);

  // Only run the timer while it is actually displayed.
  useEffect(() => {
    if (!presentation.showTimer) return;
    const id = window.setInterval(() => tick((value) => value + 1), 500);
    return () => window.clearInterval(id);
  }, [presentation.showTimer]);

  const caption = presentation.showCaption ? captionTail(dictation.interimTranscript ?? "") : "";

  // "Can't hear you": the mic has been effectively silent for a while and
  // nothing has been transcribed — usually a muted headset or wrong input.
  const lastSoundAt = useRef(Date.now());
  useEffect(() => {
    lastSoundAt.current = Date.now();
  }, [sessionId]);
  if (Math.max(0, ...(audio?.bars ?? [])) > SOUND_THRESHOLD) lastSoundAt.current = Date.now();
  const silent =
    presentation.tone === "listening" &&
    !caption &&
    Date.now() - lastSoundAt.current > SILENCE_HINT_MS &&
    Date.now() - new Date(dictation.startedAt ?? Date.now()).getTime() > SILENCE_HINT_MS;
  const label = silent ? "Can't hear you" : presentation.label;
  const detail = silent ? "Check your microphone" : presentation.detail;
  const showWaveform = presentation.showWaveform && settings.showWaveform;

  return (
    <div
      key={sessionId ?? "idle"}
      className={`overlay-surface overlay-surface--${presentation.tone}`}
      style={{ opacity: settings.overlayOpacity / 100 }}
      data-tauri-drag-region
      role="status"
      aria-live="polite"
    >
      <span className={`overlay-dot overlay-dot--${presentation.tone}`} aria-hidden="true">
        {presentation.showSpinner && <span className="overlay-spinner" />}
      </span>

      <div className="overlay-body" data-tauri-drag-region>
        {caption ? (
          <span className="overlay-caption" title={dictation.interimTranscript}>
            <bdi>{caption}</bdi>
          </span>
        ) : (
          <span className="overlay-line" data-tauri-drag-region>
            <span className="overlay-label">{label}</span>
            {detail && <span className="overlay-detail">{detail}</span>}
          </span>
        )}
      </div>

      {showWaveform && (
        <div className="overlay-wave">
          <AudioVisualizer bars={audio?.bars ?? []} />
        </div>
      )}
      {presentation.showTimer && <span className="overlay-timer">{elapsed(dictation.startedAt)}</span>}

      <div className="overlay-actions">
        {presentation.showFinish && (
          <button type="button" className="overlay-button overlay-button--finish" aria-label="Finish dictation" onClick={() => void native.finish()}>
            <Check size={14} />
          </button>
        )}
        {presentation.tone === "error" && (
          <button type="button" className="overlay-button" aria-label="Open History" title="Open History" onClick={() => void native.showHistory()}>
            <FileText size={14} />
          </button>
        )}
        {presentation.tone !== "success" && (
          <button
            type="button"
            className="overlay-button overlay-button--cancel overlay-button--hover"
            aria-label={presentation.tone === "error" ? "Dismiss" : "Cancel dictation"}
            onClick={() => void native.cancel()}
          >
            <X size={14} />
          </button>
        )}
      </div>
    </div>
  );
}
