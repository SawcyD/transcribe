import type { ButtonHTMLAttributes, ReactNode } from "react";
import { Button as MemoraButton } from "@memora/ui";

type ButtonVariant = "primary" | "secondary" | "ghost" | "danger";

interface ButtonProps extends ButtonHTMLAttributes<HTMLButtonElement> {
  variant?: ButtonVariant;
  icon?: ReactNode;
  staticMotion?: boolean;
}

export function Button({ variant = "secondary", icon, staticMotion = false, className = "", children, ...props }: ButtonProps) {
  const memoraVariant = variant === "secondary" ? "standard" : variant === "ghost" ? "subtle" : variant;
  return (
    <MemoraButton
      variant={memoraVariant}
      static={staticMotion}
      className={`voiceflow-button voiceflow-button--${variant} ${className}`}
      {...props}
    >
      {icon}
      {children}
    </MemoraButton>
  );
}
