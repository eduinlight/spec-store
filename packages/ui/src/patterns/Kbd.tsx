import type { ComponentPropsWithRef } from "react";

export type KbdTone = "light" | "inverse";

export interface KbdProps extends ComponentPropsWithRef<"kbd"> {
  tone?: KbdTone;
}

export function Kbd({ tone = "light", style, children, ...rest }: KbdProps) {
  const inverse = tone === "inverse";
  return (
    <kbd
      {...rest}
      style={{
        display: "inline-flex",
        alignItems: "center",
        height: inverse ? undefined : 20,
        padding: "0 6px",
        borderRadius: 3,
        background: inverse ? "rgba(255,255,255,.2)" : "var(--neutral-50)",
        boxShadow: inverse ? "none" : "inset 0 0 0 1px var(--neutral-300)",
        color: inverse ? "inherit" : "var(--neutral-600)",
        font: "500 11px/20px var(--font-mono)",
        ...style,
      }}
    >
      {children}
    </kbd>
  );
}
