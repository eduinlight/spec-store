import type { CSSProperties } from "react";

export type FieldSize = "large" | "medium" | "small";

export const fieldSizes: Record<
  FieldSize,
  { height: number; padX: number; fs: number; lh: string }
> = {
  large: { height: 40, padX: 12, fs: 16, lh: "24px" },
  medium: { height: 36, padX: 12, fs: 14, lh: "22px" },
  small: { height: 24, padX: 8, fs: 12, lh: "18px" },
};

export interface FieldShellOptions {
  size?: FieldSize;
  state?: "default" | "hover" | "focus" | "pressed" | "disabled";
  disabled?: boolean;
  error?: boolean;
}

export function fieldShell({
  size = "medium",
  state = "default",
  disabled,
  error,
}: FieldShellOptions): CSSProperties {
  const s = fieldSizes[size];
  let border = "var(--neutral-300)";
  let extra = "";
  if (error) {
    border = "var(--danger-600)";
  } else if (state === "focus") {
    border = "var(--primary-600)";
    extra = ", var(--focus-ring)";
  } else if (state === "hover") {
    border = "var(--primary-500)";
  }
  if (disabled) {
    border = "var(--neutral-200)";
  }
  return {
    display: "flex",
    alignItems: "center",
    gap: 8,
    height: s.height,
    padding: `0 ${s.padX}px`,
    boxSizing: "border-box",
    borderRadius: 4,
    background: disabled ? "var(--neutral-50)" : "#fff",
    boxShadow: `inset 0 0 0 1px ${border}${extra}`,
    font: `400 ${s.fs}px/${s.lh} var(--font-sans)`,
    color: disabled ? "var(--neutral-300)" : "var(--neutral-600)",
    cursor: disabled ? "not-allowed" : "text",
    transition: "box-shadow .15s",
  };
}

export const inputReset: CSSProperties = {
  flex: 1,
  minWidth: 0,
  border: "none",
  outline: "none",
  background: "transparent",
  font: "inherit",
  color: "inherit",
  padding: 0,
};

export function Chevron({ up = false, size = 12 }: { up?: boolean; size?: number }) {
  return (
    <svg
      aria-hidden="true"
      width={size}
      height={size}
      viewBox="0 0 12 12"
      style={{
        flexShrink: 0,
        transform: up ? "rotate(180deg)" : "none",
        transition: "transform .15s",
      }}
    >
      <path d="M2.5 4.5 6 8l3.5-3.5" fill="none" stroke="currentColor" strokeWidth="1.1" />
    </svg>
  );
}
