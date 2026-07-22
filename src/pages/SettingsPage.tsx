import { disable, enable, isEnabled } from "@tauri-apps/plugin-autostart";
import { relaunch } from "@tauri-apps/plugin-process";
import { check, type Update } from "@tauri-apps/plugin-updater";
import { useEffect, useMemo, useState } from "react";
import { ComboBox } from "@memora/ui";
import { Button } from "../components/common/Button";
import { ContentDialog } from "../components/fluent/ContentDialog";
import { InfoBar } from "../components/fluent/InfoBar";
import { SettingsRow, SettingsSection } from "../components/fluent/SettingsRow";
import { ToggleSwitch } from "../components/fluent/ToggleSwitch";
import { defaultSettings, isTauri, native } from "../lib/native";
import type { AppSettings, CleanupStyle, CredentialStatus, OverlayPosition } from "../types/models";
import { validateSettings } from "./settingsValidation";

type UpdateStatus =
  | { kind: "idle" }
  | { kind: "checking" }
  | { kind: "current" }
  | { kind: "available"; update: Update }
  | { kind: "installing"; version: string }
  | { kind: "error"; message: string };

export function SettingsPage() {
  const [settings, setSettings] = useState<AppSettings>(defaultSettings);
  const [credentials, setCredentials] = useState<CredentialStatus>({ deepgram: false, cleanup: false, assistant: false });
  const [launchOnStartup, setLaunchOnStartup] = useState(false);
  const [credentialDialog, setCredentialDialog] = useState<"deepgram" | "cleanup" | null>(null);
  const [secret, setSecret] = useState("");
  const [clearingHistory, setClearingHistory] = useState(false);
  const [resetting, setResetting] = useState(false);
  const [report, setReport] = useState<string | null>(null);
  const [notice, setNotice] = useState<string | null>(null);
  const [saved, setSaved] = useState(false);
  const [updateStatus, setUpdateStatus] = useState<UpdateStatus>({ kind: "idle" });

  const errors = useMemo(() => validateSettings(settings), [settings]);
  const valid = Object.keys(errors).length === 0;

  useEffect(() => {
    const startup = isTauri() ? isEnabled() : Promise.resolve(false);
    void Promise.all([native.settings(), native.credentialStatus(), startup]).then(
      ([stored, status, startupEnabled]) => {
        setSettings(stored);
        setCredentials(status);
        setLaunchOnStartup(startupEnabled);
      },
    );
  }, []);

  const update = <K extends keyof AppSettings>(key: K, value: AppSettings[K]) =>
    setSettings((current) => ({ ...current, [key]: value }));

  const save = async () => {
    if (!valid) return;
    const pending: Promise<unknown>[] = [native.saveSettings(settings)];
    if (isTauri()) pending.push(launchOnStartup ? enable() : disable());
    await Promise.all(pending);
    setSaved(true);
    window.setTimeout(() => setSaved(false), 2000);
  };

  const storeCredential = async () => {
    if (!credentialDialog || !secret.trim()) return;
    await native.setCredential(credentialDialog, secret.trim());
    setSecret("");
    setCredentialDialog(null);
    setCredentials(await native.credentialStatus());
  };

  const checkForUpdates = async () => {
    if (!isTauri()) return;
    setUpdateStatus({ kind: "checking" });
    try {
      const update = await check();
      setUpdateStatus(update ? { kind: "available", update } : { kind: "current" });
    } catch (error) {
      setUpdateStatus({ kind: "error", message: error instanceof Error ? error.message : String(error) });
    }
  };

  const installUpdate = async (update: Update) => {
    setUpdateStatus({ kind: "installing", version: update.version });
    try {
      await update.downloadAndInstall();
      await relaunch();
    } catch (error) {
      setUpdateStatus({ kind: "error", message: error instanceof Error ? error.message : String(error) });
    }
  };

  return (
    <div className="page">
      <header className="page-header">
        <h1>Settings</h1>
        <p className="page-header__meta">
          <span>Application behaviour, providers, privacy, and diagnostics.</span>
        </p>
      </header>

      {!valid && <InfoBar severity="error" title="Some settings are invalid" message={Object.values(errors)[0]} />}
      {notice && <InfoBar severity="success" title="Settings" message={notice} />}

      <SettingsSection title="General">
        <SettingsRow
          label="Start VoiceFlow with Windows"
          description="Keep dictation shortcuts available after you sign in."
          action={<ToggleSwitch label="Start VoiceFlow with Windows" checked={launchOnStartup} onChange={setLaunchOnStartup} />}
        />
        <SettingsRow
          label="Minimize to system tray"
          description="Hide the window from the taskbar when it is minimized."
          action={<ToggleSwitch label="Minimize to system tray" checked={settings.minimizeToTray} onChange={(value) => update("minimizeToTray", value)} />}
        />
        <SettingsRow
          label="Close window to system tray"
          description="When off, closing the window exits VoiceFlow and stops the shortcuts."
          action={<ToggleSwitch label="Close window to system tray" checked={settings.closeToTray} onChange={(value) => update("closeToTray", value)} />}
        />
        <SettingsRow
          label="Show notifications"
          description="Report insertion results and errors while the window is hidden."
          action={<ToggleSwitch label="Show notifications" checked={settings.showNotifications} onChange={(value) => update("showNotifications", value)} />}
        />
        <SettingsRow
          label="Theme"
          description="Follow the Windows theme by default, or choose a fixed appearance."
          action={
            <ComboBox
              label="Theme"
              value={settings.theme}
              onChange={(theme) => {
                update("theme", theme);
                document.documentElement.dataset.theme = theme === "system" ? "" : theme;
                document.querySelector(".memora-ui-root")?.setAttribute("data-memora-theme", theme);
              }}
              options={[{ value: "system", label: "Use system setting" }, { value: "light", label: "Light" }, { value: "dark", label: "Dark" }]}
            />
          }
        />
      </SettingsSection>

      <SettingsSection title="Updates" description="Updates are downloaded only when you choose Install, then verified with VoiceFlow’s signing key.">
        <SettingsRow
          label="VoiceFlow updates"
          description={
            updateStatus.kind === "current"
              ? "You are running the latest available version."
              : updateStatus.kind === "available"
                ? `Version ${updateStatus.update.version} is ready to install.`
                : "Check GitHub Releases for a signed update."
          }
          action={
            <Button
              variant="secondary"
              disabled={!isTauri() || updateStatus.kind === "checking" || updateStatus.kind === "installing"}
              onClick={() => void checkForUpdates()}
            >
              {updateStatus.kind === "checking" ? "Checking…" : "Check for updates"}
            </Button>
          }
        />
        {updateStatus.kind === "installing" && (
          <InfoBar severity="informational" title={`Installing VoiceFlow ${updateStatus.version}`} message="Downloading the signed update. VoiceFlow will restart when it is ready." />
        )}
        {updateStatus.kind === "error" && <InfoBar severity="error" title="Could not check for updates" message={updateStatus.message} />}
      </SettingsSection>

      <SettingsSection title="Overlay">
        <SettingsRow
          label="Show recording overlay"
          action={<ToggleSwitch label="Show recording overlay" checked={settings.showOverlay} onChange={(value) => update("showOverlay", value)} />}
        />
        <SettingsRow
          label="Show waveform"
          action={<ToggleSwitch label="Show waveform" checked={settings.showWaveform} onChange={(value) => update("showWaveform", value)} />}
        />
        <SettingsRow
          label="Play start and stop tones"
          action={<ToggleSwitch label="Play start and stop tones" checked={settings.playTones} onChange={(value) => update("playTones", value)} />}
        />
        <SettingsRow
          label="Overlay position"
          action={
            <select
              className="settings-combo"
              aria-label="Overlay position"
              value={settings.overlayPosition}
              onChange={(event) => update("overlayPosition", event.target.value as OverlayPosition)}
            >
              <option value="bottom_center">Bottom center</option>
              <option value="bottom_right">Bottom right</option>
              <option value="top_center">Top center</option>
              <option value="top_right">Top right</option>
            </select>
          }
        />
        <SettingsRow
          label="Overlay opacity"
          action={
            <span className="numeric-field">
              <input
                type="number"
                aria-label="Overlay opacity percentage"
                min={40}
                max={100}
                value={settings.overlayOpacity}
                onChange={(event) => update("overlayOpacity", Number(event.target.value))}
              />
              <small>%</small>
            </span>
          }
        />
      </SettingsSection>

      <SettingsSection
        title="Providers"
        description="Audio goes to Deepgram. Cleaned text goes only to the endpoint below, and only when AI cleanup is on."
      >
        <SettingsRow
          label="Transcription provider"
          action={<span className="settings-row__value">Deepgram</span>}
        />
        <SettingsRow
          label="Deepgram credential"
          description="Stored in Windows Credential Manager. It is never displayed back to this interface."
          action={
            <>
              <span className="settings-row__value">{credentials.deepgram ? "Configured" : "Not configured"}</span>
              <Button variant="secondary" onClick={() => setCredentialDialog("deepgram")}>
                Configure
              </Button>
              <Button
                variant="secondary"
                disabled={!credentials.deepgram}
                onClick={async () => {
                  await native.deleteCredential("deepgram");
                  setCredentials(await native.credentialStatus());
                }}
              >
                Remove
              </Button>
            </>
          }
        />
        <SettingsRow
          label="Cleanup provider"
          action={
            <input
              className="settings-input"
              aria-label="Cleanup endpoint"
              value={settings.cleanupEndpoint}
              onChange={(event) => update("cleanupEndpoint", event.target.value)}
            />
          }
        />
        <SettingsRow
          label="Cleanup credential"
          action={
            <>
              <span className="settings-row__value">{credentials.cleanup ? "Configured" : "Not configured"}</span>
              <Button variant="secondary" onClick={() => setCredentialDialog("cleanup")}>
                Configure
              </Button>
              <Button
                variant="secondary"
                disabled={!credentials.cleanup}
                onClick={async () => {
                  await native.deleteCredential("cleanup");
                  setCredentials(await native.credentialStatus());
                }}
              >
                Remove
              </Button>
            </>
          }
        />
        <SettingsRow
          label="Cleanup model"
          action={
            <input
              className="settings-input"
              aria-label="Cleanup model"
              value={settings.cleanupModel}
              onChange={(event) => update("cleanupModel", event.target.value)}
            />
          }
        />
        <SettingsRow
          label="Default cleanup style"
          action={
            <select
              className="settings-combo"
              aria-label="Default cleanup style"
              value={settings.cleanupStyle}
              onChange={(event) => update("cleanupStyle", event.target.value as CleanupStyle)}
            >
              <option value="balanced">Balanced</option>
              <option value="casual">Casual</option>
              <option value="developer">Developer</option>
              <option value="code_literal">Code literal</option>
            </select>
          }
        />
      </SettingsSection>

      <SettingsSection
        title="Privacy"
        description="VoiceFlow never records while idle, never stores provider keys in SQLite, and never logs transcript text by default."
      >
        <SettingsRow
          label="Store dictation history"
          action={<ToggleSwitch label="Store dictation history" checked={settings.saveHistory} onChange={(value) => update("saveHistory", value)} />}
        />
        <SettingsRow
          label="Store raw transcript"
          description="The unmodified provider output, before any cleanup."
          action={<ToggleSwitch label="Store raw transcript" checked={settings.storeRawTranscript} onChange={(value) => update("storeRawTranscript", value)} />}
        />
        <SettingsRow
          label="Store normalized transcript"
          action={<ToggleSwitch label="Store normalized transcript" checked={settings.storeNormalizedTranscript} onChange={(value) => update("storeNormalizedTranscript", value)} />}
        />
        <SettingsRow
          label="Store cleaned transcript"
          action={<ToggleSwitch label="Store cleaned transcript" checked={settings.storeCleanedTranscript} onChange={(value) => update("storeCleanedTranscript", value)} />}
        />
        <SettingsRow
          label="Include transcript text in logs"
          description="Off by default. Turn this on only while reproducing a problem."
          action={<ToggleSwitch label="Include transcript text in logs" checked={settings.includeTranscriptInLogs} onChange={(value) => update("includeTranscriptInLogs", value)} />}
        />
        <SettingsRow
          label="Allow screen context"
          description="Controls whether the assistant may capture your screen at all."
          action={<ToggleSwitch label="Allow screen context" checked={settings.assistantAllowScreenContext} onChange={(value) => update("assistantAllowScreenContext", value)} />}
        />
        <SettingsRow
          label="Confirm before pasting again"
          action={<ToggleSwitch label="Confirm before pasting again" checked={settings.confirmPasteAgain} onChange={(value) => update("confirmPasteAgain", value)} />}
        />
      </SettingsSection>

      <SettingsSection title="History">
        <SettingsRow
          label="Automatically delete history"
          description="Transcripts older than this are removed after each dictation. Zero keeps everything."
          action={
            <span className="numeric-field">
              <input
                type="number"
                aria-label="History retention in days"
                min={0}
                max={3650}
                value={settings.historyRetentionDays}
                onChange={(event) => update("historyRetentionDays", Number(event.target.value))}
              />
              <small>days</small>
            </span>
          }
        />
        <SettingsRow
          label="Maximum history size"
          description="Zero means unlimited."
          action={
            <span className="numeric-field">
              <input
                type="number"
                aria-label="Maximum stored transcripts"
                min={0}
                value={settings.maxHistoryEntries}
                onChange={(event) => update("maxHistoryEntries", Number(event.target.value))}
              />
              <small>entries</small>
            </span>
          }
        />
        <SettingsRow
          label="Clear history"
          description="Removes every stored transcript. Settings and credentials are kept."
          action={
            <Button variant="secondary" onClick={() => setClearingHistory(true)}>
              Clear history
            </Button>
          }
        />
      </SettingsSection>

      <SettingsSection title="Advanced">
        <SettingsRow
          label="Enable debug logging"
          description="Records more detail. Takes effect after restarting VoiceFlow."
          action={<ToggleSwitch label="Enable debug logging" checked={settings.debugLogging} onChange={(value) => update("debugLogging", value)} />}
        />
        <SettingsRow
          label="Open log folder"
          action={
            <Button variant="secondary" onClick={() => void native.openDataFolder("logs")}>
              Open
            </Button>
          }
        />
        <SettingsRow
          label="Open database folder"
          action={
            <Button variant="secondary" onClick={() => void native.openDataFolder("database")}>
              Open
            </Button>
          }
        />
        <SettingsRow
          label="Export diagnostic report"
          description="A summary of your configuration. Contains no transcript text and no credentials."
          action={
            <Button variant="secondary" onClick={async () => setReport(await native.diagnosticReport())}>
              Generate
            </Button>
          }
        />
        <SettingsRow
          label="Reset VoiceFlow"
          description="Restores every setting to its default. History and credentials are kept."
          action={
            <Button variant="secondary" onClick={() => setResetting(true)}>
              Reset
            </Button>
          }
        />
      </SettingsSection>

      <div className="command-row">
        <Button variant="primary" disabled={!valid} onClick={() => void save()}>
          {saved ? "Saved" : "Save changes"}
        </Button>
      </div>

      <ContentDialog
        open={updateStatus.kind === "available"}
        title={updateStatus.kind === "available" ? `VoiceFlow ${updateStatus.update.version} is ready` : "VoiceFlow update"}
        primaryText="Install and restart"
        onPrimary={() => {
          if (updateStatus.kind === "available") void installUpdate(updateStatus.update);
        }}
        closeText="Not now"
        onClose={() => setUpdateStatus({ kind: "idle" })}
      >
        {updateStatus.kind === "available" && (
          <div className="update-dialog">
            <p>VoiceFlow found a signed update. It will download in the background, install, and then restart the app.</p>
            {updateStatus.update.body && <pre className="update-dialog__notes">{updateStatus.update.body}</pre>}
          </div>
        )}
      </ContentDialog>

      <ContentDialog
        open={credentialDialog !== null}
        title={credentialDialog === "deepgram" ? "Configure Deepgram" : "Configure cleanup provider"}
        primaryText="Save credential"
        primaryDisabled={!secret.trim()}
        onPrimary={() => void storeCredential()}
        onClose={() => {
          setCredentialDialog(null);
          setSecret("");
        }}
      >
        <p>The key is written directly to Windows Credential Manager. VoiceFlow never stores it in the database and never reads it back into this window.</p>
        <div className="dialog-form">
          <label>
            <span>API key</span>
            <input
              className="settings-input"
              type="password"
              autoComplete="off"
              value={secret}
              onChange={(event) => setSecret(event.target.value)}
            />
          </label>
        </div>
      </ContentDialog>

      <ContentDialog
        open={clearingHistory}
        title="Clear all history?"
        primaryText="Clear history"
        destructive
        onPrimary={async () => {
          const removed = await native.clearHistory();
          setClearingHistory(false);
          setNotice(`Removed ${removed} ${removed === 1 ? "transcript" : "transcripts"}.`);
        }}
        onClose={() => setClearingHistory(false)}
      >
        <p>Every stored transcript and all four of its stages will be permanently deleted. This cannot be undone.</p>
      </ContentDialog>

      <ContentDialog
        open={resetting}
        title="Reset VoiceFlow?"
        primaryText="Reset settings"
        destructive
        onPrimary={async () => {
          const stored = await native.saveSettings({ ...defaultSettings, lastPage: "/settings" });
          setSettings(stored);
          setResetting(false);
          setNotice("Settings restored to their defaults.");
        }}
        onClose={() => setResetting(false)}
      >
        <p>All settings return to their defaults, including your shortcuts. Your transcript history and stored credentials are not affected.</p>
      </ContentDialog>

      <ContentDialog
        open={report !== null}
        title="Diagnostic report"
        primaryText="Copy"
        onPrimary={() => {
          if (report) void native.copyText(report);
          setReport(null);
        }}
        closeText="Close"
        onClose={() => setReport(null)}
      >
        <pre className="diagnostic-report">{report}</pre>
      </ContentDialog>
    </div>
  );
}
