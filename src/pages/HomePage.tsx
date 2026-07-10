import { ArrowRight, ClipboardCopy, Mic2, Play, Sparkles, Timer, Waves } from "lucide-react";
import { useEffect, useState } from "react";
import { useNavigate } from "react-router-dom";
import { useAppStore } from "../app/useAppStore";
import { Button } from "../components/common/Button";
import { StatusBadge } from "../components/common/StatusBadge";
import { native } from "../lib/native";
import type { AppSettings, CredentialStatus, DashboardStats } from "../types/models";

export function HomePage() {
  const navigate = useNavigate();
  const { dictation, lastTranscript } = useAppStore();
  const [settings, setSettings] = useState<AppSettings | null>(null);
  const [credentials, setCredentials] = useState<CredentialStatus>({ deepgram: false, cleanup: false });
  const [stats, setStats] = useState<DashboardStats>({ dailyWords: 0, dailySessions: 0, estimatedMinutesSaved: 0 });

  useEffect(() => {
    void Promise.all([native.settings(), native.credentialStatus(), native.stats()]).then(([nextSettings, nextCredentials, nextStats]) => {
      setSettings(nextSettings);
      setCredentials(nextCredentials);
      setStats(nextStats);
    });
  }, []);

  const listening = dictation.state === "listening_push_to_talk" || dictation.state === "listening_hands_free";
  const ready = credentials.deepgram && dictation.state === "idle";

  return (
    <div className="page page--home">
      <header className="page-header stagger-1">
        <div><span className="eyebrow">LOCAL DICTATION WORKSPACE</span><h1>Ready when the thought lands.</h1><p>Capture technical ideas without breaking focus, then place clean text exactly where you were working.</p></div>
        <StatusBadge state={dictation.state} />
      </header>

      {!credentials.deepgram && (
        <section className="setup-callout stagger-2">
          <div className="icon-well"><Sparkles size={19} /></div>
          <div><strong>Connect transcription to begin</strong><p>Your key is stored in Windows Credential Manager and never exposed to the webview.</p></div>
          <Button variant="primary" onClick={() => navigate("/settings")} icon={<ArrowRight size={16} />}>Open settings</Button>
        </section>
      )}

      <section className="hero-card stagger-2">
        <div className="hero-orbit"><span className={listening ? "mic-core mic-core--active" : "mic-core"}><Mic2 size={31} /></span><i /><i /></div>
        <div className="hero-copy">
          <span className="eyebrow">PUSH TO TALK</span>
          <h2>{listening ? "Listening to your voice" : ready ? "Hold the shortcut and speak" : "Provider setup required"}</h2>
          <p>{dictation.interimTranscript || "VoiceFlow captures first, connects second, so the beginning of your sentence stays intact."}</p>
          <div className="shortcut-row"><kbd>Ctrl</kbd><span>+</span><kbd>Win</kbd><small>Hold to record · release to finish</small></div>
          <p className="shortcut-hint"><kbd>Ctrl</kbd> + <kbd>Win</kbd> + <kbd>Space</kbd> toggles hands-free. <kbd>Ctrl</kbd> + <kbd>Win</kbd> + <kbd>Alt</kbd> enters Command Mode.</p>
          <div className="hero-actions">
            <Button
              variant={listening ? "danger" : "primary"}
              disabled={!credentials.deepgram && !listening}
              onClick={() => void (listening ? native.finish() : native.start())}
              icon={listening ? <Waves size={16} /> : <Play size={16} />}
            >{listening ? "Finish now" : "Test dictation"}</Button>
            {!listening && <Button variant="secondary" disabled={!credentials.deepgram} onClick={() => void native.start("hands_free")} icon={<Mic2 size={16} />}>Hands-free</Button>}
          </div>
        </div>
      </section>

      <section className="metric-grid stagger-3">
        <article><span><Waves size={17} />Words today</span><strong>{stats.dailyWords.toLocaleString()}</strong><small>{stats.dailySessions} sessions</small></article>
        <article><span><Timer size={17} />Time reclaimed</span><strong>{stats.estimatedMinutesSaved.toFixed(1)}m</strong><small>at 40 WPM typing</small></article>
        <article><span><Mic2 size={17} />Input</span><strong className="metric-text">{settings?.microphoneName ?? "System default"}</strong><small>{settings?.language ?? "en-US"} · mono PCM</small></article>
      </section>

      <section className="last-card stagger-4">
        <div className="section-heading"><div><span className="eyebrow">LATEST OUTPUT</span><h2>Last transcript</h2></div>{lastTranscript && <time>{new Date(lastTranscript.createdAt).toLocaleTimeString([], { hour: "numeric", minute: "2-digit" })}</time>}</div>
        {lastTranscript ? (
          <><blockquote>{lastTranscript.finalTranscript}</blockquote><div className="last-actions"><span>{lastTranscript.applicationName ?? "Unknown app"} · {lastTranscript.insertionStatus}</span><Button variant="ghost" onClick={() => void native.copyText(lastTranscript.finalTranscript)} icon={<ClipboardCopy size={16} />}>Copy</Button><Button variant="ghost" onClick={() => navigate("/history")}>View details</Button></div></>
        ) : <p className="empty-copy">Your first successful dictation will appear here.</p>}
      </section>
    </div>
  );
}
