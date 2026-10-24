import type { CSSProperties, MouseEvent, ReactNode } from "react";
import { useInteraction } from "../taptap/interaction";

export interface SidebarItemProps {
  label: string;
  selected?: boolean;
  icon?: ReactNode;
  badge?: number | string;
  href?: string;
  onClick?: (event: MouseEvent<HTMLElement>) => void;
  style?: CSSProperties;
}

export function SidebarItem({
  label,
  selected = false,
  icon,
  badge,
  href,
  onClick,
  style,
}: SidebarItemProps) {
  const [state, handlers] = useInteraction();
  const hovered = state === "hover" || state === "pressed";
  const focused = state === "focus";
  const rowStyle: CSSProperties = {
    position: "relative",
    display: "flex",
    alignItems: "center",
    gap: 12,
    width: "100%",
    height: 40,
    boxSizing: "border-box",
    padding: "0 20px",
    margin: 0,
    border: "none",
    outline: "none",
    textAlign: "left",
    textDecoration: "none",
    cursor: "pointer",
    fontFamily: "var(--font-sans)",
    fontSize: 14,
    lineHeight: "22px",
    fontWeight: selected ? 500 : 400,
    background: selected ? "var(--primary-100)" : hovered ? "var(--neutral-50)" : "transparent",
    color: selected ? "var(--primary-700)" : hovered ? "var(--neutral-700)" : "var(--neutral-600)",
    boxShadow: focused ? "inset 0 0 0 2px var(--primary-500)" : "none",
    transition: "background .15s, color .15s",
    ...style,
  };
  const content = (
    <>
      <span
        aria-hidden="true"
        style={{
          position: "absolute",
          left: 0,
          top: 8,
          bottom: 8,
          width: 3,
          borderRadius: "0 2px 2px 0",
          background: selected ? "var(--primary-600)" : "transparent",
        }}
      />
      {icon ? (
        <span
          aria-hidden="true"
          style={{
            display: "flex",
            color: selected ? "var(--primary-600)" : "var(--neutral-500)",
          }}
        >
          {icon}
        </span>
      ) : null}
      <span style={{ flex: 1, minWidth: 0 }}>{label}</span>
      {badge !== undefined && badge !== "" ? " " : null}
      {badge !== undefined && badge !== "" ? (
        <span
          style={{
            minWidth: 20,
            height: 18,
            boxSizing: "border-box",
            padding: "0 6px",
            borderRadius: 100,
            background: selected ? "var(--primary-200)" : "var(--neutral-100)",
            color: selected ? "var(--primary-700)" : "var(--neutral-500)",
            font: "400 12px/18px var(--font-numeric)",
            textAlign: "center",
          }}
        >
          {badge}
        </span>
      ) : null}
    </>
  );
  if (href) {
    return (
      <a
        href={href}
        aria-current={selected ? "page" : undefined}
        onClick={onClick}
        {...handlers}
        style={rowStyle}
      >
        {content}
      </a>
    );
  }
  return (
    <button
      type="button"
      aria-current={selected ? "page" : undefined}
      onClick={onClick}
      {...handlers}
      style={rowStyle}
    >
      {content}
    </button>
  );
}
