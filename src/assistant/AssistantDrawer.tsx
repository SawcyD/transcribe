import { Copy, Eye, MessageCircle, Send, Volume2, VolumeX, X } from "lucide-react";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { useEffect, useRef, useState } from "react";
import { MiniMarkdown } from "../lib/miniMarkdown";
import { events, native } from "../lib/native";
import { speak, stopSpeaking } from "../lib/speech";
import type { AssistantConversationTurn, ScreenContext } from "../types/models";

type Status = "ready" | "thinking" | "streaming" | "completed" | "error";

function nativeErrorMessage(reason: unknown) {
  if (reason instanceof Error) return reason.message;
  if (typeof reason === "string") return reason;
  return "VoiceFlow couldn't start that request. Add an Assistant API key in Settings, then try again.";
}

/**
 * The assistant's answer bubble. It floats just above the recording dock
 * (purple = the assistant speaking, blue dock = you), shows what it looked at,
 * renders the answer, and lets you follow up, copy, or have it read aloud.
 */
export function AssistantDrawer() {
  const [context, setContext] = useState<ScreenContext | null>(null);
  const [question, setQuestion] = useState("");
  const [answer, setAnswer] = useState("");
  const [status, setStatus] = useState<Status>("ready");
  const [error, setError] = useState("");
  const [reply, setReply] = useState("");
  const [replying, setReplying] = useState(false);
  const [speakResponses, setSpeakResponses] = useState(false);
  const [placement, setPlacement] = useState<"above" | "below">("above");
  const [copied, setCopied] = useState(false);
  const requestId = useRef<string | null>(null);
  const answerRef = useRef("");
  const contextRef = useRef<ScreenContext | null>(null);
  const conversation = useRef<AssistantConversationTurn[]>([]);
  const completedRequest = useRef<string | null>(null);
  const speakResponsesRef = useRef(false);

  const reset = (nextContext: ScreenContext | null) => {
    stopSpeaking();
    contextRef.current = nextContext;
    setContext(nextContext);
    setAnswer("");
    answerRef.current = "";
    conversation.current = [];
    setError("");
    setReplying(false);
    setReply("");
    requestId.current = null;
  };

  const send = async (message: string, attach: ScreenContext | null) => {
    const history = conversation.current;
    setQuestion(message);
    setAnswer("");
    answerRef.current = "";
    setError("");
    setStatus("thinking");
    setReplying(false);
    conversation.current = [...history, { role: "user" as const, content: message }].slice(-12);
    try {
      requestId.current = await native.askAssistant(message, attach, history);
    } catch (reason) {
      setStatus("error");
      setError(nativeErrorMessage(reason));
    }
  };

  useEffect(() => {
    let mounted = true;
    void Promise.all([native.pendingAssistantContext(), native.pendingAssistantVoicePrompt()]).then(([screen, spoken]) => {
      if (!mounted || (!screen && !spoken)) return;
      reset(screen);
      if (spoken) void send(spoken, screen);
    });
    const cleanups = Promise.all([
      events.assistantScreenContext((value) => {
        if (!mounted) return;
        reset(value);
        setQuestion("");
        setStatus("ready");
        setReplying(true);
      }),
      events.assistantVoicePrompt(({ prompt, screenContext }) => {
        if (!mounted) return;
        reset(screenContext);
        void send(prompt, screenContext);
      }),
      events.assistantPlacement((value) => {
        if (mounted) setPlacement(value);
      }),
      events.assistantState((value) => {
        if (!mounted || (requestId.current !== null && value.requestId !== requestId.current)) return;
        requestId.current = value.requestId;
        if (value.state === "error") {
          setError(value.message ?? "The assistant couldn't finish that request.");
          setStatus("error");
          return;
        }
        if (value.state === "completed" && completedRequest.current !== value.requestId && answerRef.current.trim()) {
          conversation.current = [...conversation.current, { role: "assistant" as const, content: answerRef.current.trim() }].slice(-12);
          completedRequest.current = value.requestId;
          if (speakResponsesRef.current) void speak(answerRef.current);
        }
        setStatus(value.state);
      }),
      events.assistantDelta((value) => {
        if (!mounted || (requestId.current !== null && value.requestId !== requestId.current)) return;
        requestId.current = value.requestId;
        answerRef.current += value.delta;
        setAnswer(answerRef.current);
      }),
    ]);
    return () => {
      mounted = false;
      void cleanups.then((items) => items.forEach((unlisten) => unlisten()));
    };
    // Subscriptions are set up once; handlers only touch refs and setters.
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);

  useEffect(() => {
    speakResponsesRef.current = speakResponses;
  }, [speakResponses]);
  useEffect(() => {
    void native.settings().then((settings) => setSpeakResponses(settings.buddySpeakResponses));
  }, []);

  const close = () => {
    stopSpeaking();
    void getCurrentWindow().hide();
  };

  // Esc closes the bubble, like any popover.
  useEffect(() => {
    const onKey = (event: KeyboardEvent) => {
      if (event.key === "Escape") close();
    };
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  });

  const busy = status === "thinking" || status === "streaming";

  const toggleSpeech = async () => {
    const next = !speakResponses;
    setSpeakResponses(next);
    const settings = await native.settings();
    await native.saveSettings({ ...settings, buddySpeakResponses: next });
    if (next && answer) void speak(answer);
    if (!next) stopSpeaking();
  };

  const copy = async () => {
    await native.copyText(answer);
    setCopied(true);
    window.setTimeout(() => setCopied(false), 1400);
  };

  const sendReply = () => {
    if (!reply.trim() || busy) return;
    // Follow-ups keep the same screenshot so "how do I fix it?" still knows
    // what "it" is.
    void send(reply.trim(), contextRef.current);
    setReply("");
  };

  const looking = context?.application ?? context?.windowTitle ?? null;

  return (
    <main className={`assistant-drawer assistant-drawer--${placement}`} aria-label="VoiceFlow Assistant">
      <section className="bubble" aria-live="polite">
        <header className="bubble__header">
          <span className={`bubble__orb${busy ? " bubble__orb--busy" : ""}`} aria-hidden="true" />
          <strong>Assistant</strong>
          {looking && (
            <span className="bubble__chip" title="The assistant can see this window">
              <Eye size={12} /> {looking}
            </span>
          )}
          <button type="button" className="bubble__icon bubble__close" aria-label="Close (Esc)" title="Close · Esc" onClick={close}>
            <X size={15} />
          </button>
        </header>

        {question && <p className="bubble__question">“{question}”</p>}

        <div className="bubble__answer">
          {answer ? (
            <MiniMarkdown text={answer} />
          ) : busy ? (
            <p className="bubble__thinking">
              {looking ? `Looking at ${looking}` : "Thinking"}
              <span className="bubble__dots" aria-hidden="true">
                <i />
                <i />
                <i />
              </span>
            </p>
          ) : !error ? (
            <p className="bubble__hint">Ask about what's on your screen.</p>
          ) : null}
          {error && <p className="bubble__error">{error}</p>}
        </div>

        {replying ? (
          <div className="bubble__composer">
            <textarea
              autoFocus
              rows={2}
              value={reply}
              placeholder={answer ? "Ask a follow-up…" : "What do you want to know?"}
              onChange={(event) => setReply(event.target.value)}
              onKeyDown={(event) => {
                if (event.key === "Enter" && !event.shiftKey) {
                  event.preventDefault();
                  sendReply();
                }
              }}
            />
            <button type="button" className="bubble__send" disabled={!reply.trim() || busy} aria-label="Send" onClick={sendReply}>
              <Send size={14} />
            </button>
          </div>
        ) : (
          <footer className="bubble__footer">
            <button type="button" className="bubble__action" disabled={busy} onClick={() => setReplying(true)}>
              <MessageCircle size={14} /> Follow up
            </button>
            <button type="button" className="bubble__icon" disabled={!answer} aria-label="Copy answer" title="Copy" onClick={() => void copy()}>
              <Copy size={14} />
            </button>
            <button
              type="button"
              className={`bubble__icon${speakResponses ? " bubble__icon--on" : ""}`}
              aria-pressed={speakResponses}
              aria-label={speakResponses ? "Stop reading answers aloud" : "Read answers aloud"}
              title={speakResponses ? "Reading aloud" : "Read aloud"}
              onClick={() => void toggleSpeech()}
            >
              {speakResponses ? <Volume2 size={14} /> : <VolumeX size={14} />}
            </button>
            {copied && <span className="bubble__toast">Copied</span>}
          </footer>
        )}
      </section>
    </main>
  );
}
