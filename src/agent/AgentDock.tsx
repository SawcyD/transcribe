import { PhoneOff } from "lucide-react";
import { useEffect, useState } from "react";
import { AudioVisualizer } from "../overlay/AudioVisualizer";
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

const HISTORY = 16;

/**
 * Voice-agent mode: a purple pill for the agent stacked above a blue pill for
 * you, mirroring the recording dock. Each shows a live waveform and the latest
 * line said.
 */
export function AgentDock() {
  const [state, setState] = useState<AgentState>("connecting");
  const [message, setMessage] = useState<string | null>(null);
  const [agentLine, setAgentLine] = useState("");
  const [userLine, setUserLine] = useState("");
  const [agentBars, setAgentBars] = useState<number[]>(() => Array(HISTORY).fill(0));
  const [micBars, setMicBars] = useState<number[]>([]);

  useEffect(() => {
    const subscriptions = Promise.all([
      events.agentState(({ state: next, message: detail }) => {
        setState(next);
        setMessage(detail);
        if (next === "connecting") {
          setAgentLine("");
          setUserLine("");
        }
      }),
      events.agentTranscript(({ role, content }) => (role === "user" ? setUserLine(content) : setAgentLine(content))),
      // Output loudness arrives ~20x a second; keep a short rolling history
      // so the agent's pill shows a moving waveform like yours.
      events.agentLevel((level) => setAgentBars((bars) => [...bars.slice(1), Math.min(1, level * 4)])),
      events.audio((payload) => {
        if (payload.sessionId === "voice-agent") setMicBars(payload.bars);
      }),
    ]);
    return () => void subscriptions.then((items) => items.forEach((unlisten) => unlisten()));
  }, []);

  const agentActive = state === "speaking";
  const userActive = state === "listening";

  return (
    <div className="agent-dock" data-tauri-drag-region>
      <div className={`agent-pill agent-pill--${state}`} data-tauri-drag-region role="status" aria-live="polite">
        <span className={`agent-pill__orb${state === "thinking" || state === "connecting" ? " agent-pill__orb--busy" : ""}`} aria-hidden="true" />
        <span className="agent-pill__text">
          {state === "error" ? (message ?? AGENT_LABELS.error) : agentLine && state !== "thinking" ? (
            <span className="agent-pill__caption">
              <bdi>{captionTail(agentLine, 140)}</bdi>
            </span>
          ) : (
            AGENT_LABELS[state]
          )}
        </span>
        <span className={`agent-pill__wave${agentActive ? "" : " agent-pill__wave--idle"}`}>
          <AudioVisualizer bars={agentBars} />
        </span>
      </div>

      <div className={`you-pill${userActive ? " you-pill--active" : ""}`} data-tauri-drag-region>
        <span className="you-pill__dot" aria-hidden="true" />
        <span className="you-pill__text">
          {userLine ? (
            <span className="agent-pill__caption">
              <bdi>{captionTail(userLine, 140)}</bdi>
            </span>
          ) : agentActive ? (
            "Agent is talking…"
          ) : (
            "Say something"
          )}
        </span>
        <span className="you-pill__wave">
          <AudioVisualizer bars={micBars} />
        </span>
        <button type="button" className="you-pill__end" aria-label="End conversation" title="End conversation" onClick={() => void native.stopVoiceAgent()}>
          <PhoneOff size={14} />
        </button>
      </div>
    </div>
  );
}
