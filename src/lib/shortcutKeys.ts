import type { ShortcutBinding } from "../types/models";

const KEY_NAMES: Record<string, string> = { ctrl: "Ctrl", control: "Ctrl", win: "Win", meta: "Win", alt: "Alt", shift: "Shift", escape: "Esc", space: "Space" };

/** Display names for a binding, e.g. ["Ctrl", "Alt", "B"]. */
export function bindingKeys(binding: ShortcutBinding | undefined): string[] {
  if (!binding) return [];
  const name = (value: string) => KEY_NAMES[value.toLowerCase()] ?? (value.length === 1 ? value.toUpperCase() : value);
  return [...binding.modifiers.map(name), ...(binding.key ? [name(binding.key)] : [])];
}
