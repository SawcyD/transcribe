import { Copy, Eye, MessageCircle, Send, Sparkles, Volume2, VolumeX, X } from "lucide-react";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { IconButton } from "@memora/ui";
import { useEffect, useRef, useState } from "react";
import { events, native } from "../lib/native";
import type { AssistantConversationTurn, ScreenContext } from "../types/models";

type Status = "ready" | "thinking" | "streaming" | "completed" | "error";

function nativeErrorMessage(reason: unknown) {
  if (reason instanceof Error) return reason.message;
  if (typeof reason === "string") return reason;
  if (reason && typeof reason === "object" && "message" in reason && typeof reason.message === "string") return reason.message;
  return "Buddy could not start that request. Add an Assistant API key in Settings, then try again.";
}

function speak(text: string) {
  if (!("speechSynthesis" in window) || !text.trim()) return;
  window.speechSynthesis.cancel();
  const utterance = new SpeechSynthesisUtterance(text);
  utterance.rate = 1.04;
  utterance.pitch = 1.03;
  window.speechSynthesis.speak(utterance);
}

export function AssistantDrawer() {
  const [context, setContext] = useState<ScreenContext | null>(null);
  const [prompt, setPrompt] = useState("Explain what I am looking at and what I should check next.");
  const [answer, setAnswer] = useState("");
  const [status, setStatus] = useState<Status>("ready");
  const [error, setError] = useState("");
  const [voicePrompt, setVoicePrompt] = useState<string | null>(null);
  const [composerOpen, setComposerOpen] = useState(false);
  const [speakResponses, setSpeakResponses] = useState(false);
  const requestId = useRef<string | null>(null);
  const answerRef = useRef("");
  const conversation = useRef<AssistantConversationTurn[]>([]);
  const completedRequest = useRef<string | null>(null);
  const speakResponsesRef = useRef(false);

  useEffect(() => {
    let mounted = true;
    void Promise.all([native.pendingAssistantContext(), native.pendingAssistantVoicePrompt()]).then(([screenContext, spokenPrompt]) => {
      if (!mounted) return;
      setContext(screenContext);
      if (spokenPrompt) setVoicePrompt(spokenPrompt);
    });
    const cleanups = Promise.all([
      events.assistantScreenContext((value) => {
        if (!mounted) return;
        setContext(value); setAnswer(""); answerRef.current = ""; conversation.current = []; setError(""); setStatus("ready"); setComposerOpen(false); requestId.current = null;
      }),
      events.assistantVoicePrompt((value) => {
        if (!mounted) return;
        setContext(null); setAnswer(""); answerRef.current = ""; conversation.current = []; setError(""); setStatus("ready"); setComposerOpen(false); requestId.current = null; setVoicePrompt(value);
      }),
      events.assistantState((value) => {
        if (!mounted || (requestId.current !== null && value.requestId !== requestId.current)) return;
        requestId.current = value.requestId;
        if (value.state === "error") { setError(value.message ?? "Buddy could not complete that request."); setStatus("error"); }
        else {
          if (value.state === "completed" && completedRequest.current !== value.requestId && answerRef.current.trim()) {
            conversation.current = [...conversation.current, { role: "assistant" as const, content: answerRef.current.trim() }].slice(-12);
            completedRequest.current = value.requestId;
            if (speakResponsesRef.current) speak(answerRef.current);
          }
          setStatus(value.state);
        }
      }),
      events.assistantDelta((value) => {
        if (!mounted || (requestId.current !== null && value.requestId !== requestId.current)) return;
        requestId.current = value.requestId;
        answerRef.current += value.delta;
        setAnswer(answerRef.current);
      }),
    ]);
    return () => { mounted = false; void cleanups.then((items) => items.forEach((unlisten) => unlisten())); };
  }, []);

  useEffect(() => { speakResponsesRef.current = speakResponses; }, [speakResponses]);
  useEffect(() => { void native.settings().then((settings) => setSpeakResponses(settings.buddySpeakResponses)); }, []);

  useEffect(() => {
    if (!voicePrompt || status !== "ready") return;
    setPrompt(voicePrompt);
    setAnswer(""); answerRef.current = ""; setError(""); setStatus("thinking"); setVoicePrompt(null);
    const history = conversation.current;
    conversation.current = [...history, { role: "user" as const, content: voicePrompt }].slice(-12);
    void native.askAssistant(voicePrompt, null, history).then((id) => { requestId.current = id; }).catch((reason) => {
      setStatus("error"); setError(nativeErrorMessage(reason));
    });
  }, [status, voicePrompt]);

  async function ask() {
    if (!prompt.trim() || status === "thinking" || status === "streaming") return;
    const message = prompt.trim();
    const history = conversation.current;
    setAnswer(""); answerRef.current = ""; setError(""); setStatus("thinking"); setComposerOpen(false);
    conversation.current = [...history, { role: "user" as const, content: message }].slice(-12);
    try { requestId.current = await native.askAssistant(message, context, history); }
    catch (reason) { setStatus("error"); setError(nativeErrorMessage(reason)); }
  }

  async function toggleSpeech() {
    const next = !speakResponses;
    setSpeakResponses(next);
    const settings = await native.settings();
    await native.saveSettings({ ...settings, buddySpeakResponses: next });
    if (!next && "speechSynthesis" in window) window.speechSynthesis.cancel();
  }

  const hasScreenContext = context !== null;
  const isBusy = status === "thinking" || status === "streaming";
  const statusLabel = isBusy ? status === "thinking" ? "Thinking" : "Responding" : status === "error" ? "Needs attention" : "Ready";
  const title = answer ? "Buddy has a thought" : hasScreenContext ? "Explain this screen" : "Ask Buddy";
  const summary = isBusy ? (hasScreenContext ? "Looking at what is on your screen" : "Thinking through your request") : prompt.trim() || "What are you working on?";

  return <main className="assistant-drawer" aria-label="VoiceFlow Assistant">
    <section className={`assistant-thought ${composerOpen ? "assistant-thought--replying" : ""}`} aria-live="polite">
      <header><div className="assistant-thought__title"><Sparkles size={12} /><strong>{title}</strong></div><IconButton label="Close Assistant" variant="subtle" onClick={() => void getCurrentWindow().hide()}><X size={16} /></IconButton></header>
      <div className="assistant-thought__body"><span className="assistant-thought__context"><Eye size={13} />{context?.application ?? "VoiceFlow Buddy"}</span><p>{answer || summary}</p>{isBusy && <span className="assistant-thought__dots" aria-label={statusLabel}><i /><i /><i /></span>}{error && <p className="assistant-thought__error">{error}</p>}</div>
      <footer><button className="assistant-thought__reply" onClick={() => setComposerOpen((open) => !open)} disabled={isBusy}><MessageCircle size={14} />{composerOpen ? "Close" : "Reply"}</button><button className="assistant-thought__copy" disabled={!answer} onClick={() => void native.copyText(answer)} aria-label="Copy Buddy response"><Copy size={14} /></button><button className="assistant-thought__copy" onClick={() => void toggleSpeech()} aria-label={speakResponses ? "Turn off spoken Buddy responses" : "Turn on spoken Buddy responses"}>{speakResponses ? <Volume2 size={14} /> : <VolumeX size={14} />}</button></footer>
      {composerOpen && <div className="assistant-thought__composer"><textarea autoFocus value={prompt} onChange={(event) => setPrompt(event.target.value)} onKeyDown={(event) => { if (event.key === "Enter" && (event.ctrlKey || event.metaKey)) { event.preventDefault(); void ask(); } }} placeholder="Reply to Buddy" /><button disabled={!prompt.trim() || isBusy} onClick={() => void ask()}><Send size={14} />Send</button></div>}
    </section>
  </main>;
}
