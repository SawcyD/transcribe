import type { ButtonHTMLAttributes, ReactNode } from "react";

type ButtonVariant = "primary" | "secondary" | "ghost" | "danger";

interface ButtonProps extends ButtonHTMLAttributes<HTMLButtonElement> {
  variant?: ButtonVariant;
  icon?: ReactNode;
  staticMotion?: boolean;
}

export function Button({ variant = "secondary", icon, staticMotion = false, className = "", children, ...props }: ButtonProps) {
  return (
    <button
      className={`button button--${variant} ${staticMotion ? "button--static" : ""} ${className}`}
      {...props}
    >
      {icon}
      {children}
    </button>
  );
}
