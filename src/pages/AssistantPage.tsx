import { AudioLines, CalendarDays, History, PenLine, MessageSquarePlus, Monitor, Send, Trash2, X } from "lucide-react";
import { useEffect, useMemo, useRef, useState } from "react";
import { Button } from "../components/common/Button";
import { PageTabs } from "../components/common/PageTabs";
import { usePageTab } from "../components/common/usePageTab";
import { ContentDialog } from "../components/fluent/ContentDialog";
import { InfoBar } from "../components/fluent/InfoBar";
import { SettingsRow, SettingsSection } from "../components/fluent/SettingsRow";
import { ToggleSwitch } from "../components/fluent/ToggleSwitch";
import { MiniMarkdown } from "../lib/miniMarkdown";
import { defaultSettings, events, native } from "../lib/native";
import { ASSISTANT_VOICES, VOICE_AGENT_VOICES, speak } from "../lib/speech";
import type { AppSettings, AssistantChatSummary, AssistantConversationTurn, CredentialStatus, ScreenContext } from "../types/models";

type Tab = "chat" | "saved";

function relativeDay(iso: string): string {
  const date = new Date(iso);
  const today = new Date();
  const days = Math.round((new Date(today.toDateString()).getTime() - new Date(date.toDateString()).getTime()) / 86_400_000);
  if (days === 0) return date.toLocaleTimeString([], { hour: "numeric", minute: "2-digit" });
  if (days === 1) return "Yesterday";
  if (days < 7) return date.toLocaleDateString([], { weekday: "long" });
  return date.toLocaleDateString([], { month: "short", day: "numeric" });
}

/** Chat up front; Flow's and the chat's settings each get their own tab. */
const ASSISTANT_TABS = [
  { id: "chat", label: "Chat" },
  { id: "flow", label: "Flow voice agent" },
  { id: "settings", label: "Chat settings" },
] as const;

export function AssistantPage() {
  const [settings, setSettings] = useState<AppSettings>(defaultSettings);
  const [credentials, setCredentials] = useState<CredentialStatus>({ deepgram: false, cleanup: false, assistant: false, search: false });
  const [section, setSection] = usePageTab(ASSISTANT_TABS);
  const [tab, setTab] = useState<Tab>("chat");
  const [turns, setTurns] = useState<AssistantConversationTurn[]>([]);
  const [chatId, setChatId] = useState<string>(() => crypto.randomUUID());
  const [prompt, setPrompt] = useState("");
  const [streaming, setStreaming] = useState("");
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [context, setContext] = useState<ScreenContext | null>(null);
  const [chats, setChats] = useState<AssistantChatSummary[]>([]);
  const [chatSearch, setChatSearch] = useState("");
  // Which key is being entered, if any.
  const [keyDialog, setKeyDialog] = useState<"assistant" | "search" | null>(null);
  const [secret, setSecret] = useState("");
  const conversation = useRef<HTMLDivElement>(null);
  const requestId = useRef<string | null>(null);
  const streamingRef = useRef("");

  const loadChats = () => void native.assistantChats().then(setChats);

  useEffect(() => {
    void native.settings().then(setSettings);
    void native.credentialStatus().then(setCredentials);
    loadChats();
  }, []);

  // Answers stream in as events for the request we started; the call itself
  // only returns the request id.
  useEffect(() => {
    const subscriptions = Promise.all([
      events.assistantDelta((event) => {
        if (event.requestId !== requestId.current) return;
        streamingRef.current += event.delta;
        setStreaming(streamingRef.current);
      }),
      events.assistantState((event) => {
        if (event.requestId !== requestId.current) return;
        if (event.state === "error") {
          setError(event.message ?? "The assistant couldn't answer.");
          setBusy(false);
          requestId.current = null;
        } else if (event.state === "completed") {
          const answer = streamingRef.current.trim();
          if (answer) setTurns((current) => [...current, { role: "assistant", content: answer }]);
          streamingRef.current = "";
          setStreaming("");
          setBusy(false);
          requestId.current = null;
          loadChats();
          if (answer && settingsRef.current.buddySpeakResponses) void speak(answer);
        }
      }),
    ]);
    return () => void subscriptions.then((items) => items.forEach((unlisten) => unlisten()));
  }, []);

  const settingsRef = useRef(settings);
  settingsRef.current = settings;

  useEffect(() => {
    conversation.current?.scrollTo({ top: conversation.current.scrollHeight, behavior: "smooth" });
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

  const newChat = () => {
    setTurns([]);
    setChatId(crypto.randomUUID());
    setContext(null);
    setError(null);
    setTab("chat");
  };

  const openChat = async (chat: AssistantChatSummary) => {
    const saved = await native.assistantChat(chat.id);
    setTurns(saved.flatMap((turn) => [
      { role: "user" as const, content: turn.question },
      { role: "assistant" as const, content: turn.answer },
    ]));
    setChatId(chat.id);
    setContext(null);
    setError(null);
    setTab("chat");
  };

  const deleteChat = async (id: string) => {
    await native.deleteAssistantChat(id);
    if (id === chatId) newChat();
    loadChats();
  };

  const send = async () => {
    const text = prompt.trim();
    if (!text || busy) return;
    setBusy(true);
    setError(null);
    streamingRef.current = "";
    setStreaming("");
    const history = turns.slice(-12);
    setTurns([...turns, { role: "user", content: text }]);
    setPrompt("");
    try {
      requestId.current = await native.askAssistant(text, context, history, chatId);
      // Screen context is single-use, so a stale capture is never silently
      // attached to a later question.
      setContext(null);
    } catch (cause) {
      setError(cause instanceof Error ? cause.message : "The assistant couldn't answer.");
      setBusy(false);
    }
  };

  const storeKey = async () => {
    if (!secret.trim() || !keyDialog) return;
    await native.setCredential(keyDialog, secret.trim());
    setSecret("");
    setKeyDialog(null);
    setCredentials(await native.credentialStatus());
  };

  const visibleChats = useMemo(() => {
    const needle = chatSearch.trim().toLowerCase();
    return needle ? chats.filter((chat) => `${chat.title} ${chat.preview}`.toLowerCase().includes(needle)) : chats;
  }, [chats, chatSearch]);

  const providerReady = credentials.assistant || credentials.cleanup;

  return (
    <div className="page assistant-page">
      <header className="page-header">
        <h1>Assistant</h1>
        <p className="page-header__meta">
          <span>
            Stuck on something? Hold your Command Mode shortcut and ask out loud — it looks at the window you're in and answers in a bubble above
            the dock. Or chat here.
          </span>
        </p>
      </header>

      <PageTabs tabs={ASSISTANT_TABS} active={section} onSelect={setSection} label="Assistant sections" />

      {!providerReady && (
        <InfoBar
          severity="warning"
          title="The assistant needs an AI provider"
          message="Add an OpenAI-compatible API key under Chat settings › Provider (or set up AI cleanup in Settings, which it can share)."
        />
      )}

      {section === "chat" && (
        <>
      <div className="assistant-tabs">
        <div className="segmented" role="tablist" aria-label="Assistant view">
          <button type="button" role="tab" aria-selected={tab === "chat"} className={`segmented__item${tab === "chat" ? " segmented__item--selected" : ""}`} onClick={() => setTab("chat")}>
            Chat
          </button>
          <button type="button" role="tab" aria-selected={tab === "saved"} className={`segmented__item${tab === "saved" ? " segmented__item--selected" : ""}`} onClick={() => setTab("saved")}>
            Saved chats{chats.length > 0 ? ` · ${chats.length}` : ""}
          </button>
        </div>
        <span className="assistant-tabs__actions">
          <button type="button" className="text-button" onClick={newChat}>
            <MessageSquarePlus size={14} aria-hidden="true" /> New chat
          </button>
          <button type="button" className="text-button" onClick={() => void native.showWhiteboard()} title="A shared board you and Flow can both draw on">
            <PenLine size={14} aria-hidden="true" /> Whiteboard
          </button>
          <button type="button" className="text-button" onClick={() => void native.showPlanner()} title="Your day plan, which Flow can read and change">
            <CalendarDays size={14} aria-hidden="true" /> Planner
          </button>
          <Button variant="primary" icon={<AudioLines size={15} />} onClick={() => void native.startVoiceAgent()} title="Spoken back-and-forth conversation · Ctrl+Alt+V">
            Talk out loud
          </Button>
        </span>
      </div>

      {tab === "chat" ? (
        <>
          <div className="chat" ref={conversation} aria-live="polite">
            {turns.length === 0 && !streaming ? (
              <div className="chat-empty">
                <strong>Ask about anything you're stuck on</strong>
                <p>Attach your screen so it can see what you see. {settings.assistantRememberChats ? "It remembers past chats, so you can say “remember when I…”." : ""}</p>
              </div>
            ) : (
              <>
                {turns.map((turn, index) => (
                  <div key={index} className={`chat-turn chat-turn--${turn.role}`}>
                    {turn.role === "user" ? <p>{turn.content}</p> : <MiniMarkdown text={turn.content} />}
                  </div>
                ))}
                {busy && (
                  <div className="chat-turn chat-turn--assistant">
                    {streaming ? <MiniMarkdown text={streaming} /> : <p className="chat-turn__thinking">Thinking…</p>}
                  </div>
                )}
              </>
            )}
          </div>

          {error && <InfoBar severity="error" title="The assistant couldn't answer" message={error} />}

          <div className="composer composer--assistant">
            {context && (
              <span className="context-chip">
                <Monitor size={14} aria-hidden="true" /> Seeing {context.application ?? "your screen"}
                <button type="button" className="context-chip__remove" aria-label="Remove screen context" onClick={() => setContext(null)}>
                  <X size={13} />
                </button>
              </span>
            )}
            <textarea
              value={prompt}
              aria-label="Ask the assistant"
              placeholder={context ? `Ask about ${context.application ?? "your screen"}…` : "Ask anything…"}
              rows={2}
              onChange={(event) => setPrompt(event.target.value)}
              onKeyDown={(event) => {
                if (event.key === "Enter" && !event.shiftKey) {
                  event.preventDefault();
                  void send();
                }
              }}
            />
            <div className="composer__actions">
              <Button variant="secondary" icon={<Monitor size={15} />} disabled={!settings.assistantAllowScreenContext || busy} onClick={() => void attachContext()}>
                {context ? "Recapture" : "Attach screen"}
              </Button>
              <small className="composer__hint">Enter to send · Shift+Enter for a new line</small>
              <Button variant="primary" icon={<Send size={15} />} disabled={!prompt.trim() || busy} onClick={() => void send()}>
                {busy ? "Thinking…" : "Send"}
              </Button>
            </div>
          </div>
        </>
      ) : (
        <section className="saved-chats">
          {chats.length > 0 && (
            <input className="settings-input saved-chats__search" placeholder="Search saved chats…" aria-label="Search saved chats" value={chatSearch} onChange={(event) => setChatSearch(event.target.value)} />
          )}
          {visibleChats.length === 0 ? (
            <div className="empty-state">
              <History size={20} aria-hidden="true" />
              <strong>{chats.length === 0 ? "No saved chats yet" : "No chats match"}</strong>
              <p>{settings.assistantRememberChats ? "Conversations you have with the assistant show up here." : "Turn on “Remember chats” in Chat settings to keep conversations."}</p>
            </div>
          ) : (
            <div className="saved-chats__list">
              {visibleChats.map((chat) => (
                <div key={chat.id} className={`saved-chat${chat.id === chatId ? " saved-chat--open" : ""}`}>
                  <button type="button" className="saved-chat__main" onClick={() => void openChat(chat)}>
                    <span className="saved-chat__top">
                      <b>{chat.title}</b>
                      <time>{relativeDay(chat.updatedAt)}</time>
                    </span>
                    <span className="saved-chat__preview">{chat.preview}</span>
                    <span className="saved-chat__meta">
                      {chat.turnCount} {chat.turnCount === 1 ? "question" : "questions"}
                      {chat.applicationName ? ` · ${chat.applicationName}` : ""}
                    </span>
                  </button>
                  <button type="button" className="icon-button icon-button--danger" aria-label={`Delete “${chat.title}”`} title="Delete chat" onClick={() => void deleteChat(chat.id)}>
                    <Trash2 size={15} />
                  </button>
                </div>
              ))}
            </div>
          )}
        </section>
      )}
        </>
      )}

      {section === "flow" && (
        <>
        <SettingsSection title="Personality and voice">
          <SettingsRow
            label="Personality"
            description="How Flow talks to you. It does the same work in every mode. Takes effect on the next conversation."
            action={
              <select className="settings-combo" aria-label="Personality" value={settings.voiceAgentPersonality} onChange={(event) => void update("voiceAgentPersonality", event.target.value as AppSettings["voiceAgentPersonality"])}>
                <option value="friendly">Friendly</option>
                <option value="smug">Smug (dry, reluctantly brilliant)</option>
                <option value="professional">Professional</option>
                <option value="hype">Hype</option>
                <option value="blunt">Blunt</option>
                <option value="mentor">Mentor</option>
              </select>
            }
          />
          <SettingsRow
            label="About you (flow.md)"
            description='A note Flow reads at the start of every conversation: your name, how you like answers, people you mention. Say "Flow, remember…" and it adds to it.'
            action={
              <Button variant="secondary" onClick={() => void native.openFlowProfile()}>
                Edit flow.md
              </Button>
            }
          />
          <SettingsRow
            label="Voice agent voice"
            description="Who you hear in a live conversation. Takes effect on the next conversation."
            action={
              <select className="settings-combo" aria-label="Voice agent voice" value={settings.voiceAgentVoice} onChange={(event) => void update("voiceAgentVoice", event.target.value)}>
                {VOICE_AGENT_VOICES.map((voice) => (
                  <option key={voice.id} value={voice.id}>
                    {voice.label}
                  </option>
                ))}
              </select>
            }
          />
          <SettingsRow
            label="Expressivity"
            description="How animated the voice agent sounds."
            action={
              <select className="settings-combo" aria-label="Expressivity" value={settings.voiceAgentExpressivity} onChange={(event) => void update("voiceAgentExpressivity", Number(event.target.value))}>
                <option value={2}>2 (more animated)</option>
                <option value={1}>1</option>
                <option value={0}>0 (default)</option>
                <option value={-1}>-1</option>
                <option value={-2}>-2 (calmer)</option>
              </select>
            }
          />
        </SettingsSection>
        <SettingsSection title="What Flow can do">
          <SettingsRow
            label={'Wake up on "Hey Flow"'}
            description="Start a conversation by saying it. Keeps the microphone open; listening happens on this PC. It uses the microphone chosen in Settings."
            action={<ToggleSwitch label="Wake up on Hey Flow" checked={settings.wakeWordEnabled} onChange={(value) => void update("wakeWordEnabled", value)} />}
          />
          <SettingsRow
            label="Let me interrupt the voice agent"
            description="Talk over it to cut it off. Use headphones: on speakers it would hear itself and reply to its own voice."
            action={<ToggleSwitch label="Let me interrupt the voice agent" checked={settings.voiceAgentInterruptible} onChange={(value) => void update("voiceAgentInterruptible", value)} />}
          />
          <SettingsRow
            label="Let the voice agent open apps and websites"
            description={'For requests like "open OBS". It can only open things, not change or delete them.'}
            action={<ToggleSwitch label="Let the voice agent open apps and websites" checked={settings.voiceAgentCanAct} onChange={(value) => void update("voiceAgentCanAct", value)} />}
          />
          <SettingsRow
            label="Web search"
            description="Lets Flow look things up. Needs a free Tavily key from tavily.com; searches are sent to Tavily."
            action={
              <span className="voice-picker">
                <span className="settings-row__value">{credentials.search ? "Configured" : "Not set"}</span>
                <Button variant="secondary" onClick={() => setKeyDialog("search")}>
                  {credentials.search ? "Replace" : "Add key"}
                </Button>
                {credentials.search && (
                  <Button variant="secondary" onClick={() => void native.deleteCredential("search").then(() => native.credentialStatus().then(setCredentials))}>
                    Remove
                  </Button>
                )}
              </span>
            }
          />
        </SettingsSection>
        </>
      )}

      {section === "settings" && (
        <>
        <SettingsSection title="Chat">
          <SettingsRow
            label="Remember chats"
            description="Saves conversations here and lets the assistant recall them later (“remember that interview I mentioned?”). Stored only on this PC."
            action={<ToggleSwitch label="Remember chats" checked={settings.assistantRememberChats} onChange={(value) => void update("assistantRememberChats", value)} />}
          />
          <SettingsRow
            label="Allow screen context"
            description="When off, the assistant can never capture your screen, including from Command Mode."
            action={<ToggleSwitch label="Allow screen context" checked={settings.assistantAllowScreenContext} onChange={(value) => void update("assistantAllowScreenContext", value)} />}
          />
          <SettingsRow
            label="Read answers aloud"
            action={<ToggleSwitch label="Read answers aloud" checked={settings.buddySpeakResponses} onChange={(value) => void update("buddySpeakResponses", value)} />}
          />
          <SettingsRow
            label="Voice"
            description="Deepgram voice used when answers are read aloud."
            action={
              <span className="voice-picker">
                <select className="settings-combo" aria-label="Assistant voice" value={settings.assistantVoice ?? ""} onChange={(event) => void update("assistantVoice", event.target.value || null)}>
                  {ASSISTANT_VOICES.map((voice, index) => (
                    <option key={voice.id} value={index === 0 ? "" : voice.id}>
                      {voice.label}
                    </option>
                  ))}
                </select>
                <Button variant="secondary" onClick={() => void speak("Hi! This is how I'll sound when I read answers to you.")}>
                  Preview
                </Button>
              </span>
            }
          />
        </SettingsSection>
        <SettingsSection
          title="Provider"
          description={credentials.assistant ? "Using the assistant's own provider below." : "No separate key: the assistant uses your AI cleanup provider. Add a key to use a different model (it needs to accept images to see your screen)."}
        >
          <SettingsRow
            label="Endpoint"
            description="Any OpenAI-compatible API."
            action={<input className="settings-input" aria-label="Assistant endpoint" value={settings.assistantEndpoint} onChange={(event) => setSettings({ ...settings, assistantEndpoint: event.target.value })} onBlur={() => void update("assistantEndpoint", settings.assistantEndpoint.trim())} />}
          />
          <SettingsRow
            label="Model"
            action={<input className="settings-input" aria-label="Assistant model" value={settings.assistantModel} onChange={(event) => setSettings({ ...settings, assistantModel: event.target.value })} onBlur={() => void update("assistantModel", settings.assistantModel.trim())} />}
          />
          <SettingsRow
            label="API key"
            description="Stored in Windows Credential Manager."
            action={
              <span className="voice-picker">
                <span className="settings-row__value">{credentials.assistant ? "Configured" : "Not set"}</span>
                <Button variant="secondary" onClick={() => setKeyDialog("assistant")}>
                  {credentials.assistant ? "Replace" : "Add key"}
                </Button>
                {credentials.assistant && (
                  <Button variant="secondary" onClick={() => void native.deleteCredential("assistant").then(() => native.credentialStatus().then(setCredentials))}>
                    Remove
                  </Button>
                )}
              </span>
            }
          />
        </SettingsSection>
        </>
      )}

      <ContentDialog open={keyDialog !== null} title={keyDialog === "search" ? "Web search key" : "Assistant API key"} primaryText="Save" primaryDisabled={!secret.trim()} onPrimary={() => void storeKey()} onClose={() => { setKeyDialog(null); setSecret(""); }}>
        <p>{keyDialog === "search" ? "Your Tavily API key, from tavily.com. It lets Flow search the web." : "Used only for the assistant."} It is never shown again after saving.</p>
        <input className="settings-input dictionary-quick-add" type="password" aria-label="API key" value={secret} onChange={(event) => setSecret(event.target.value)} />
      </ContentDialog>
    </div>
  );
}
