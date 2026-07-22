import { Check, Copy, FileText, X } from "lucide-react";
import { useEffect, useState } from "react";
import { useAppStore } from "../app/useAppStore";
import { native } from "../lib/native";
import { AudioVisualizer } from "./AudioVisualizer";
import { overlayPresentation } from "./OverlayState";
import { useTones } from "./useTones";
import type { AppSettings } from "../types/models";
import { defaultSettings } from "../lib/native";

function elapsed(startedAt: string | null | undefined): string {
  if (!startedAt) return "00:00";
  const seconds = Math.max(0, Math.floor((Date.now() - new Date(startedAt).getTime()) / 1000));
  return `${String(Math.floor(seconds / 60)).padStart(2, "0")}:${String(seconds % 60).padStart(2, "0")}`;
}

export function RecordingOverlay() {
  const { dictation, audio, lastTranscript } = useAppStore();
  const [settings, setSettings] = useState<AppSettings>(defaultSettings);
  const [, tick] = useState(0);

  useEffect(() => {
    void native.settings().then(setSettings);
  }, [dictation.state]);

  const presentation = overlayPresentation(dictation.state, lastTranscript?.applicationName);
  useTones(dictation.state, settings.playTones);

  // Only run the timer while it is actually displayed.
  useEffect(() => {
    if (!presentation.showTimer) return;
    const id = window.setInterval(() => tick((value) => value + 1), 500);
    return () => window.clearInterval(id);
  }, [presentation.showTimer]);

  const showWaveform = presentation.showWaveform && settings.showWaveform;

  return (
    <div
      className={`overlay-surface overlay-surface--${presentation.tone}`}
      style={{ opacity: settings.overlayOpacity / 100 }}
      data-tauri-drag-region
      role="status"
      aria-live="polite"
    >
      <span className={`overlay-dot overlay-dot--${presentation.tone}`} aria-hidden="true" />

      <div className="overlay-body" data-tauri-drag-region>
        <div className="overlay-line" data-tauri-drag-region>
          <span className="overlay-label">{presentation.label}</span>
          {presentation.showTimer && <span className="overlay-timer">{elapsed(dictation.startedAt)}</span>}
          {presentation.showSpinner && <span className="overlay-spinner" aria-hidden="true" />}
        </div>
        {presentation.detail && <span className="overlay-detail">{presentation.detail}</span>}
        {showWaveform && <AudioVisualizer bars={audio?.bars ?? []} />}
      </div>

      {/* Controls stay hidden until hover or keyboard focus, per the spec. */}
      <div className="overlay-actions">
        {presentation.showFinish && (
          <button type="button" className="overlay-button overlay-button--finish" aria-label="Finish dictation" onClick={() => void native.finish()}>
            <Check size={14} />
          </button>
        )}
        {presentation.tone === "error" && lastTranscript && (
          <>
            <button
              type="button"
              className="overlay-button"
              aria-label="Copy recovered transcript"
              onClick={() => void native.copyText(lastTranscript.finalTranscript)}
            >
              <Copy size={14} />
            </button>
            <button type="button" className="overlay-button" aria-label="Open transcript in History" onClick={() => void native.showHistory()}>
              <FileText size={14} />
            </button>
          </>
        )}
        <button type="button" className="overlay-button overlay-button--cancel" aria-label="Cancel dictation" onClick={() => void native.cancel()}>
          <X size={14} />
        </button>
      </div>
    </div>
  );
}
