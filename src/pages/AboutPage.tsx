import { SettingsRow, SettingsSection } from "../components/fluent/SettingsRow";

export function AboutPage() {
  return (
    <div className="page">
      <header className="page-header">
        <h1>About</h1>
        <p className="page-header__meta">
          <span>Local-first voice dictation for Windows.</span>
        </p>
      </header>

      <SettingsSection title="VoiceFlow">
        <SettingsRow label="Version" action={<span className="settings-row__value">0.1.0</span>} />
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
