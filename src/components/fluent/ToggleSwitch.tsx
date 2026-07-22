import { ToggleSwitch as MemoraToggleSwitch } from "@memora/ui";

interface ToggleSwitchProps {
  checked: boolean;
  onChange: (checked: boolean) => void;
  /** Accessible name; supply when the visible label lives in the parent row. */
  label: string;
  disabled?: boolean;
}

/** Fluent toggle with the On/Off text Windows 11 places beside the control. */
export function ToggleSwitch({ checked, onChange, label, disabled = false }: ToggleSwitchProps) {
  return (
    <span className="voiceflow-toggle">
      <MemoraToggleSwitch checked={checked} onChange={onChange} disabled={disabled} label={label} />
      <span aria-hidden="true">{checked ? "On" : "Off"}</span>
    </span>
  );
}
