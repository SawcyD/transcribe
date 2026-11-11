import { PhoneOff } from "lucide-react";
import { useEffect, useRef, useState } from "react";
import { captionTail } from "../overlay/OverlayState";
import { events, native, type AgentState } from "../lib/native";

const AGENT_LABELS: Record<AgentState, string> = {
  connecting: "Connecting…",
  listening: "Listening",
  thinking: "Thinking…",
  speaking: "Speaking",
  ended: "Conversation ended",
  error: "Something went wrong",
};

/**
 * Voice-agent mode: an orb that rises from the taskbar and swells with
 * whoever is talking (your mic while it listens, its own voice while it
 * speaks), above one caption line showing the latest thing said.
 */
export function AgentDock() {
  const [state, setState] = useState<AgentState>("connecting");
  const [message, setMessage] = useState<string | null>(null);
  const [line, setLine] = useState<{ role: string; content: string } | null>(null);
  // Bumped per conversation so the orb replays its entrance.
  const [session, setSession] = useState(0);
  const orb = useRef<HTMLDivElement>(null);
  const live = useRef<AgentState>("connecting");

  useEffect(() => {
    // Loudness arrives ~30x a second. It drives a CSS variable directly so the
    // orb moves without re-rendering React on every tick.
    const setLevel = (level: number) => orb.current?.style.setProperty("--level", Math.max(0, Math.min(1, level)).toFixed(3));
    const subscriptions = Promise.all([
      events.agentState(({ state: next, message: detail }) => {
        live.current = next;
        setState(next);
        setMessage(detail);
        if (next !== "speaking" && next !== "listening") setLevel(0);
        if (next === "connecting") {
          setLine(null);
          setSession((value) => value + 1);
        }
      }),
      events.agentTranscript(setLine),
      events.agentLevel((level) => {
        if (live.current === "speaking") setLevel(level * 4);
      }),
      events.audio((payload) => {
        if (payload.sessionId !== "voice-agent" || live.current !== "listening" || payload.bars.length === 0) return;
        setLevel((payload.bars.reduce((sum, bar) => sum + bar, 0) / payload.bars.length) * 1.6);
      }),
    ]);
    return () => void subscriptions.then((items) => items.forEach((unlisten) => unlisten()));
  }, []);

  const label = state === "error" ? (message ?? AGENT_LABELS.error) : AGENT_LABELS[state];
  const showLine = line && (state === "listening" || state === "speaking");
  const speaker = state === "speaking" || (showLine && line.role !== "user") ? "agent" : state === "listening" ? "user" : "idle";

  return (
    <div className={`agent-dock${state === "ended" ? " agent-dock--ended" : ""}`} key={session} data-tauri-drag-region>
      <div className="agent-orb-wrap" data-tauri-drag-region>
        <div ref={orb} className={`agent-orb agent-orb--${state}`} aria-hidden="true" />
      </div>
      <div className="agent-caption" data-tauri-drag-region role="status" aria-live="polite">
        <span className={`agent-caption__dot agent-caption__dot--${speaker}`} aria-hidden="true" />
        <span className="agent-caption__text">
          {showLine ? (
            <span className="agent-caption__line">
              <bdi>{captionTail(line.content, 140)}</bdi>
            </span>
          ) : (
            <span className="agent-caption__label" key={state}>
              {label}
            </span>
          )}
        </span>
        <button type="button" className="agent-caption__end" aria-label="End conversation" title="End conversation" onClick={() => void native.stopVoiceAgent()}>
          <PhoneOff size={14} />
        </button>
      </div>
    </div>
  );
}
