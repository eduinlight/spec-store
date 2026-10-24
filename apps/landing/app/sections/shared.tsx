import { cn } from "@workspace/ui/lib/utils";
import type { ComponentPropsWithoutRef, CSSProperties, ReactNode } from "react";

export const CONTAINER: CSSProperties = { maxWidth: 1200, margin: "0 auto" };

export const TEXT_LINK = "text-[var(--primary-700)] hover:text-[var(--primary-500)]";

export const CARD_TITLE: CSSProperties = { margin: 0, font: "500 18px/26px var(--font-sans)" };

export const CARD_BODY: CSSProperties = {
  margin: 0,
  fontSize: 14,
  lineHeight: "22px",
  color: "var(--neutral-600)",
};

export const SECTION_TITLE: CSSProperties = {
  margin: 0,
  font: "500 clamp(26px, 5vw, 34px)/1.25 var(--font-sans)",
  textWrap: "balance",
};

export const BODY_LARGE: CSSProperties = {
  margin: 0,
  fontSize: 16,
  lineHeight: "26px",
  color: "var(--neutral-600)",
};

type ActionLinkProps = ComponentPropsWithoutRef<"a"> & {
  variant?: "primary" | "outline";
  size?: "medium" | "large";
  block?: boolean;
};

export function ActionLink({
  variant = "primary",
  size = "medium",
  block = false,
  className,
  ...rest
}: ActionLinkProps) {
  return (
    <a
      {...rest}
      className={cn(
        "items-center justify-center whitespace-nowrap rounded-[4px] font-medium no-underline transition-colors focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-[var(--primary-600)]",
        block ? "flex w-full" : "inline-flex",
        size === "large"
          ? "h-10 min-w-[90px] px-5 text-base leading-6"
          : "h-9 min-w-[82px] px-3 text-sm leading-[22px]",
        variant === "primary"
          ? "bg-[var(--primary-600)] text-white hover:bg-[var(--primary-500)] hover:text-white active:bg-[var(--primary-700)]"
          : "bg-white text-[var(--neutral-600)] shadow-[inset_0_0_0_1px_var(--neutral-300)] hover:text-[var(--primary-500)] hover:shadow-[inset_0_0_0_1px_var(--primary-500)] active:text-[var(--primary-700)] active:shadow-[inset_0_0_0_1px_var(--primary-700)]",
        className,
      )}
    />
  );
}

export function Eyebrow({
  children,
  color = "var(--primary-700)",
}: {
  children: ReactNode;
  color?: string;
}) {
  return (
    <span
      style={{
        font: "500 12px/18px var(--font-sans)",
        color,
        letterSpacing: ".08em",
        textTransform: "uppercase",
      }}
    >
      {children}
    </span>
  );
}

export function SectionHeading({
  eyebrow,
  title,
  maxWidth = 640,
}: {
  eyebrow: string;
  title: string;
  maxWidth?: number;
}) {
  return (
    <div style={{ display: "flex", flexDirection: "column", gap: 12, maxWidth, marginBottom: 40 }}>
      <Eyebrow>{eyebrow}</Eyebrow>
      <h2 style={SECTION_TITLE}>{title}</h2>
    </div>
  );
}

export function Terminal({
  title,
  children,
  style,
}: {
  title?: string;
  children: ReactNode;
  style?: CSSProperties;
}) {
  return (
    <div
      style={{
        background: "var(--neutral-700)",
        color: "#E1E1E1",
        borderRadius: 8,
        fontFamily: "var(--font-mono)",
        overflowX: "auto",
        minWidth: 0,
        ...style,
      }}
    >
      {title ? (
        <div style={{ display: "flex", gap: 6, marginBottom: 12 }}>
          <span style={{ width: 10, height: 10, borderRadius: "50%", background: "#4B4B4B" }} />
          <span style={{ width: 10, height: 10, borderRadius: "50%", background: "#4B4B4B" }} />
          <span style={{ width: 10, height: 10, borderRadius: "50%", background: "#4B4B4B" }} />
          <span
            style={{
              marginLeft: 8,
              fontSize: 12,
              lineHeight: "10px",
              color: "var(--neutral-500)",
            }}
          >
            {title}
          </span>
        </div>
      ) : null}
      <div style={{ whiteSpace: "pre", width: "max-content", minWidth: "100%" }}>{children}</div>
    </div>
  );
}
