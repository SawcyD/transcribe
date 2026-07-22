import type { ReactNode } from "react";
import { InfoBar as MemoraInfoBar } from "@memora/ui";

type Severity = "informational" | "success" | "warning" | "error";

interface InfoBarProps {
  severity?: Severity;
  title: string;
  message?: string;
  /** Optional trailing action, e.g. a "Configure provider" button. */
  action?: ReactNode;
}

/** Inline status banner. Mirrors WinUI InfoBar rather than a dashboard callout. */
export function InfoBar({ severity = "informational", title, message, action }: InfoBarProps) {
  const tone = severity === "informational" ? "info" : severity;
  return <MemoraInfoBar title={title} message={message} action={action} tone={tone} />;
}
