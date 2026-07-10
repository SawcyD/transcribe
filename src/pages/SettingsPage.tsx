import { Check, KeyRound, Mic2, Save, ShieldCheck, SlidersHorizontal } from "lucide-react";
import { useEffect, useMemo, useState, type ReactNode } from "react";
import { Button } from "../components/common/Button";
import { defaultSettings, native } from "../lib/native";
import type { AppSettings, CredentialStatus } from "../types/models";
import { validateSettings } from "./settingsValidation";

export function SettingsPage() {
  const [settings, setSettings] = useState<AppSettings>(defaultSettings);
  const [credentials, setCredentials] = useState<CredentialStatus>({ deepgram: false, cleanup: false });
  const [microphones, setMicrophones] = useState<string[]>([]);
  const [deepgramKey, setDeepgramKey] = useState("");
  const [cleanupKey, setCleanupKey] = useState("");
  const [saved, setSaved] = useState(false);
  const errors = useMemo(() => validateSettings(settings), [settings]);
  const valid = Object.keys(errors).length === 0;

  useEffect(() => {
    void Promise.all([native.settings(), native.credentialStatus(), native.microphones()]).then(([stored, status, devices]) => {
      setSettings(stored);
      setCredentials(status);
      setMicrophones(devices);
    });
  }, []);

  const update = <K extends keyof AppSettings>(key: K, value: AppSettings[K]) => setSettings((current) => ({ ...current, [key]: value }));
  const save = async () => {
    if (!valid) return;
    const pending: Promise<unknown>[] = [native.saveSettings(settings)];
    if (deepgramKey.trim()) pending.push(native.setCredential("deepgram", deepgramKey.trim()));
    if (cleanupKey.trim()) pending.push(native.setCredential("cleanup", cleanupKey.trim()));
    await Promise.all(pending);
    setDeepgramKey("");
    setCleanupKey("");
    setCredentials(await native.credentialStatus());
    setSaved(true);
    window.setTimeout(() => setSaved(false), 1_800);
  };

  return (
    <div className="page page--settings">
      <header className="page-header"><div><span className="eyebrow">NATIVE CONFIGURATION</span><h1>Settings</h1><p>Provider secrets stay in Windows Credential Manager. Non-secret behavior is stored in the local SQLite database.</p></div><Button variant="primary" disabled={!valid} icon={saved ? <Check size={16} /> : <Save size={16} />} onClick={() => void save()}>{saved ? "Saved" : "Save changes"}</Button></header>

      <section className="settings-section"><header><span className="icon-well"><KeyRound size={18} /></span><div><h2>Providers</h2><p>Audio goes to Deepgram; cleaned text goes only to the endpoint below when enabled.</p></div></header><div className="form-grid">
        <Field label="Deepgram API key" hint={credentials.deepgram ? "Stored securely · enter a value only to replace it" : "Required for streaming transcription"}><input type="password" autoComplete="off" value={deepgramKey} onChange={(event) => setDeepgramKey(event.target.value)} placeholder={credentials.deepgram ? "••••••••••••••••" : "dg_…"} /></Field>
        <Field label="Deepgram model"><input value={settings.transcriptionModel} onChange={(event) => update("transcriptionModel", event.target.value)} /></Field>
        <Field label="Language"><select value={settings.language} onChange={(event) => update("language", event.target.value)}><option value="en-US">English (US)</option><option value="en-GB">English (UK)</option><option value="multi">Multilingual</option></select></Field>
        <Field label="Cleanup API key" hint={credentials.cleanup ? "Stored securely · enter a value only to replace it" : "Optional; deterministic cleanup remains available"}><input type="password" autoComplete="off" value={cleanupKey} onChange={(event) => setCleanupKey(event.target.value)} placeholder={credentials.cleanup ? "••••••••••••••••" : "Provider key"} /></Field>
        <Field label="Cleanup endpoint" error={errors.cleanupEndpoint}><input value={settings.cleanupEndpoint} onChange={(event) => update("cleanupEndpoint", event.target.value)} /></Field>
        <Field label="Cleanup model"><input value={settings.cleanupModel} onChange={(event) => update("cleanupModel", event.target.value)} /></Field>
        <Field label="Cleanup style" hint="Controls how much rewriting is allowed"><select value={settings.cleanupStyle} onChange={(event) => update("cleanupStyle", event.target.value as AppSettings["cleanupStyle"])}><option value="balanced">Balanced</option><option value="casual">Keep casual voice</option><option value="developer">Developer prose</option><option value="code_literal">Code literal</option></select></Field>
        <Field label="Auto-apply transform" hint="Preview remains available in Transforms; this runs after cleanup before paste"><select value={settings.autoApplyTransform ?? ""} onChange={(event) => update("autoApplyTransform", event.target.value || null)}><option value="">Off</option><option value="polish">Polish</option><option value="prompt_engineer">Prompt Engineer</option><option value="turn_into_list">Turn Into List</option></select></Field>
      </div><Toggle label="AI cleanup" description="Send normalized text to the configured cleanup provider." checked={settings.cleanupEnabled} onChange={(value) => update("cleanupEnabled", value)} /></section>

      <section className="settings-section"><header><span className="icon-well"><Mic2 size={18} /></span><div><h2>Audio</h2><p>Capture uses the device native sample rate and downmixes to mono PCM16.</p></div></header><div className="form-grid">
        <Field label="Microphone"><select value={settings.microphoneName ?? ""} onChange={(event) => update("microphoneName", event.target.value || null)}><option value="">System default</option>{microphones.map((name) => <option value={name} key={name}>{name}</option>)}</select></Field>
        <Field label="Noise floor" suffix="dB" error={errors.noiseFloorDb}><input type="number" value={settings.noiseFloorDb} min={-90} max={-10} onChange={(event) => update("noiseFloorDb", Number(event.target.value))} /></Field>
        <Field label="Session limit" suffix="minutes" error={errors.sessionLimitMinutes}><input type="number" value={settings.sessionLimitMinutes} min={1} max={120} onChange={(event) => update("sessionLimitMinutes", Number(event.target.value))} /></Field>
      </div><Toggle label="Save recordings" description="Off by default. Failed processing keeps audio in memory only for the active recovery flow." checked={settings.saveAudio} onChange={(value) => update("saveAudio", value)} /></section>

      <section className="settings-section"><header><span className="icon-well"><SlidersHorizontal size={18} /></span><div><h2>Insertion and privacy</h2><p>Clipboard restoration is best-effort because some target applications read pasted text asynchronously.</p></div></header><div className="form-grid">
        <Field label="Paste delay" suffix="ms" error={errors.pasteDelayMs}><input type="number" value={settings.pasteDelayMs} min={40} max={2000} onChange={(event) => update("pasteDelayMs", Number(event.target.value))} /></Field>
      </div><Toggle label="Restore text clipboard" description="Restore the previous text value after the target has accepted the paste." checked={settings.restoreClipboard} onChange={(value) => update("restoreClipboard", value)} /><Toggle label="Save transcript history" description="Store all four transcript stages locally in SQLite." checked={settings.saveHistory} onChange={(value) => update("saveHistory", value)} /><Toggle label="Allow “press enter”" description="Disabled by default. Only an exact phrase at the end of dictation can trigger Enter." checked={settings.pressEnterEnabled} onChange={(value) => update("pressEnterEnabled", value)} /></section>

      <section className="privacy-note"><ShieldCheck size={18} /><p><strong>Privacy boundary:</strong> VoiceFlow Dev never records while idle, never stores provider keys in SQLite, and never logs full transcript contents by default.</p></section>
    </div>
  );
}

function Field({ label, hint, error, suffix, children }: { label: string; hint?: string; error?: string; suffix?: string; children: ReactNode }) {
  return <label className="field"><span>{label}</span><div className={suffix ? "input-with-suffix" : ""}>{children}{suffix && <small>{suffix}</small>}</div>{(error ?? hint) && <em className={error ? "field-error" : ""}>{error ?? hint}</em>}</label>;
}

function Toggle({ label, description, checked, onChange }: { label: string; description: string; checked: boolean; onChange: (value: boolean) => void }) {
  return <label className="toggle-row"><span><strong>{label}</strong><small>{description}</small></span><input type="checkbox" checked={checked} onChange={(event) => onChange(event.target.checked)} /><i aria-hidden="true" /></label>;
}
