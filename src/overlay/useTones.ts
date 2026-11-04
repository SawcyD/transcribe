import { useEffect, useRef } from "react";
import { isTauri, native } from "../lib/native";
import type { DictationState } from "../types/models";

type Tone = "start" | "stop" | "success" | "error";

/** Short sine blips. Deliberately quiet — this fires while the user is working. */
const TONES: Record<Tone, { frequency: number; durationMs: number }> = {
  start: { frequency: 660, durationMs: 90 },
  stop: { frequency: 520, durationMs: 90 },
  success: { frequency: 880, durationMs: 110 },
  error: { frequency: 300, durationMs: 220 },
};

const TONE_FOR_STATE: Partial<Record<DictationState, Tone>> = {
  listening_push_to_talk: "start",
  listening_hands_free: "start",
  finalizing_audio: "stop",
  completed: "success",
  error: "error",
};

/**
 * Plays a short tone on the dictation transitions that matter.
 * (WebAudio is only the browser-dev fallback.)
 *
 * The AudioContext is created lazily on the first tone, because browsers refuse
 * to start one before a user gesture and creating it eagerly logs a warning.
 */
export function useTones(state: DictationState, enabled: boolean) {
  const contextRef = useRef<AudioContext | null>(null);
  const previous = useRef<DictationState>(state);

  useEffect(() => {
    const from = previous.current;
    previous.current = state;
    if (!enabled || from === state) return;

    const tone = TONE_FOR_STATE[state];
    if (!tone) return;

    // In the app, tones play natively on the output device chosen in
    // Settings › Audio; the WebView can only use the Windows default.
    if (isTauri()) {
      void native.playTone(tone).catch(() => undefined);
      return;
    }

    try {
      contextRef.current ??= new AudioContext();
      const context = contextRef.current;
      const { frequency, durationMs } = TONES[tone];
      const oscillator = context.createOscillator();
      const gain = context.createGain();
      oscillator.frequency.value = frequency;
      oscillator.type = "sine";
      // Ramp rather than switch, so the blip has no audible click.
      gain.gain.setValueAtTime(0, context.currentTime);
      gain.gain.linearRampToValueAtTime(0.06, context.currentTime + 0.01);
      gain.gain.linearRampToValueAtTime(0, context.currentTime + durationMs / 1000);
      oscillator.connect(gain).connect(context.destination);
      oscillator.start();
      oscillator.stop(context.currentTime + durationMs / 1000);
    } catch {
      // Audio output is optional; never let it interrupt dictation.
    }
  }, [state, enabled]);

  useEffect(() => () => void contextRef.current?.close(), []);
}
