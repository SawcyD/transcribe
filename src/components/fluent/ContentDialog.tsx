import type { ReactNode } from "react";
import { ContentDialog as MemoraContentDialog } from "@memora/ui";

interface ContentDialogProps {
  open: boolean;
  title: string;
  children?: ReactNode;
  primaryText: string;
  onPrimary: () => void;
  closeText?: string;
  onClose: () => void;
  destructive?: boolean;
  primaryDisabled?: boolean;
}

/** VoiceFlow's compatibility layer over Memora's accessible Acrylic dialog. */
export function ContentDialog({
  open,
  title,
  children,
  primaryText,
  onPrimary,
  closeText = "Cancel",
  onClose,
  destructive = false,
  primaryDisabled = false,
}: ContentDialogProps) {
  return (
    <MemoraContentDialog
      open={open}
      title={title}
      primaryText={primaryText}
      cancelText={closeText}
      destructive={destructive}
      onCancel={onClose}
      onPrimary={primaryDisabled ? undefined : onPrimary}
    >
      {children}
    </MemoraContentDialog>
  );
}
