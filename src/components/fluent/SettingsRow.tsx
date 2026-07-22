import type { ReactNode } from "react";
import { SettingsRow as MemoraSettingsRow, SettingsSection as MemoraSettingsSection } from "@memora/ui";

interface SettingsSectionProps {
  title: string;
  description?: string;
  children: ReactNode;
}

/** Keeps the existing VoiceFlow page API while delegating the surface to Memora. */
export function SettingsSection({ title, description, children }: SettingsSectionProps) {
  return (
    <section className="voiceflow-settings-group">
      <h2 className="voiceflow-settings-group__title">{title}</h2>
      {description && <p className="voiceflow-settings-group__description">{description}</p>}
      <MemoraSettingsSection>{children}</MemoraSettingsSection>
    </section>
  );
}

interface SettingsRowProps {
  icon?: ReactNode;
  label: string;
  description?: string;
  action?: ReactNode;
  onClick?: () => void;
}

export function SettingsRow({ icon, label, description, action, onClick }: SettingsRowProps) {
  const row = <MemoraSettingsRow title={label} description={description} control={action} />;
  if (!onClick) return row;
  return <button type="button" className="voiceflow-settings-row-button" onClick={onClick}>{icon}{row}</button>;
}
