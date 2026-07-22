import { getVersion } from "@tauri-apps/api/app";
import { useEffect, useState } from "react";
import { SettingsRow, SettingsSection } from "../components/fluent/SettingsRow";
import { isTauri } from "../lib/native";

export function AboutPage() {
  const [version, setVersion] = useState("0.1.1");

  useEffect(() => {
    if (!isTauri()) return;
    void getVersion().then(setVersion).catch(() => undefined);
  }, []);

  return (
    <div className="page">
      <header className="page-header">
        <h1>About</h1>
        <p className="page-header__meta">
          <span>Local-first voice dictation for Windows.</span>
        </p>
      </header>

      <SettingsSection title="VoiceFlow">
        <SettingsRow label="Version" action={<span className="settings-row__value tabular-value">{version}</span>} />
        <SettingsRow label="Update channel" action={<span className="settings-row__value">Stable · GitHub Releases</span>} />
        <SettingsRow label="Transcription" action={<span className="settings-row__value">Deepgram Nova-3</span>} />
        <SettingsRow
          label="Credential storage"
          description="Provider keys are held in Windows Credential Manager and are never returned to the interface."
          action={<span className="settings-row__value">Windows Credential Manager</span>}
        />
      </SettingsSection>
    </div>
  );
}
