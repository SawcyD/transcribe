import { useCallback, useEffect, useRef, type ReactNode } from "react";
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
  // Memora moves focus to the dialog frame whenever `onCancel` changes
  // identity. Callers pass inline functions, so every keystroke in a field
  // re-rendered, changed it, and pulled focus out of the field. Hand Memora
  // one function that never changes and always calls the latest `onClose`.
  const latestClose = useRef(onClose);
  useEffect(() => {
    latestClose.current = onClose;
  });
  const close = useCallback(() => latestClose.current(), []);

  return (
    <MemoraContentDialog
      open={open}
      title={title}
      primaryText={primaryText}
      cancelText={closeText}
      destructive={destructive}
      onCancel={close}
      onPrimary={primaryDisabled ? undefined : onPrimary}
    >
      {children}
    </MemoraContentDialog>
  );
}
