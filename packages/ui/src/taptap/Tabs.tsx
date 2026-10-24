import { type CSSProperties, type KeyboardEvent, type ReactNode, useRef, useState } from "react";

export type TabsKey = string | number;

export interface TabsItem {
  label: string;
  value?: TabsKey;
  icon?: ReactNode;
  disabled?: boolean;
  closable?: boolean;
}

export interface TabsProps {
  items?: TabsItem[];
  value?: TabsKey;
  onChange?: (value: TabsKey) => void;
  type?: "line" | "card";
  size?: "default" | "small";
  bordered?: boolean;
  centered?: boolean;
  onClose?: (value: TabsKey) => void;
  style?: CSSProperties;
}

const keyOf = (item: TabsItem): TabsKey => item.value ?? item.label;

export const Tabs = ({
  items = [],
  value,
  onChange,
  type = "line",
  size = "default",
  bordered = true,
  centered = false,
  onClose,
  style,
}: TabsProps) => {
  const [hover, setHover] = useState<number | null>(null);
  const refs = useRef<Array<HTMLDivElement | null>>([]);
  const small = size === "small";
  const fs = small ? 12 : 14;
  const lh = small ? "18px" : "22px";
  const card = type === "card";

  const enabled = items.flatMap((item, index) => (item.disabled ? [] : [index]));
  const selectedIndex = items.findIndex((item) => keyOf(item) === value);
  const tabStop = enabled.includes(selectedIndex) ? selectedIndex : (enabled[0] ?? -1);

  const select = (index: number) => {
    const item = items[index];
    if (item && !item.disabled) {
      onChange?.(keyOf(item));
    }
  };

  const moveTo = (index: number | undefined) => {
    if (index === undefined) {
      return;
    }
    refs.current[index]?.focus();
    select(index);
  };

  const onKeyDown = (event: KeyboardEvent<HTMLDivElement>) => {
    const focused = refs.current.indexOf(document.activeElement as HTMLDivElement | null);
    const position = enabled.indexOf(focused >= 0 ? focused : tabStop);
    if (event.key === "ArrowRight") {
      moveTo(enabled[(position + 1) % enabled.length]);
    } else if (event.key === "ArrowLeft") {
      moveTo(enabled[(position - 1 + enabled.length) % enabled.length]);
    } else if (event.key === "Home") {
      moveTo(enabled[0]);
    } else if (event.key === "End") {
      moveTo(enabled[enabled.length - 1]);
    } else {
      return;
    }
    event.preventDefault();
  };

  const containerStyle: CSSProperties = card
    ? {
        display: "inline-flex",
        gap: 4,
        padding: 4,
        background: bordered ? "var(--neutral-50)" : "transparent",
        borderRadius: 4,
        justifyContent: centered ? "center" : "flex-start",
        maxWidth: "100%",
        overflowX: "auto",
        ...style,
      }
    : {
        display: "flex",
        gap: 32,
        borderBottom: "1px solid var(--neutral-200)",
        justifyContent: centered ? "center" : "flex-start",
        overflowX: "auto",
        ...style,
      };

  return (
    <div role="tablist" onKeyDown={onKeyDown} style={containerStyle}>
      {items.map((item, index) => {
        const key = keyOf(item);
        const active = key === value;
        const hovered = hover === index && !item.disabled;
        const base: CSSProperties = {
          display: "flex",
          alignItems: "center",
          gap: 4,
          flexShrink: 0,
          whiteSpace: "nowrap",
          font: `400 ${fs}px/${lh} var(--font-sans)`,
          cursor: item.disabled ? "not-allowed" : "pointer",
        };
        const tabStyle: CSSProperties = card
          ? {
              ...base,
              height: small ? 30 : 34,
              padding: small ? "0 12px" : "0 16px",
              borderRadius: 2,
              background: active || hovered ? "#fff" : "transparent",
              boxShadow:
                bordered && (active || hovered) ? "inset 0 0 0 1px var(--neutral-200)" : "none",
              color: item.disabled
                ? "var(--neutral-400)"
                : active || hovered
                  ? "var(--primary-600)"
                  : "var(--neutral-600)",
            }
          : {
              ...base,
              padding: small ? "8px 0" : "10px 0",
              marginBottom: -1,
              borderBottom: `2px solid ${active ? "var(--primary-600)" : "transparent"}`,
              color: item.disabled
                ? "var(--neutral-600)"
                : active
                  ? "var(--primary-600)"
                  : hovered
                    ? "var(--neutral-400)"
                    : "var(--neutral-600)",
              opacity: item.disabled ? 0.5 : 1,
            };
        return (
          <div
            key={key}
            ref={(node) => {
              refs.current[index] = node;
            }}
            role="tab"
            aria-selected={active}
            aria-disabled={item.disabled || undefined}
            tabIndex={index === tabStop ? 0 : -1}
            onClick={() => select(index)}
            onKeyDown={(event) => {
              if (event.key === "Enter" || event.key === " ") {
                event.preventDefault();
                select(index);
              }
            }}
            onMouseEnter={() => setHover(index)}
            onMouseLeave={() => setHover(null)}
            style={tabStyle}
          >
            {item.icon}
            {item.label}
            {card && item.closable && (
              <button
                type="button"
                tabIndex={-1}
                aria-label={`Close ${item.label}`}
                onClick={(event) => {
                  event.stopPropagation();
                  onClose?.(key);
                }}
                onKeyDown={(event) => {
                  if (event.key === "Enter" || event.key === " ") {
                    event.preventDefault();
                    event.stopPropagation();
                    onClose?.(key);
                  }
                }}
                style={{
                  display: "inline-flex",
                  marginLeft: 4,
                  padding: 0,
                  border: "none",
                  background: "none",
                  cursor: "inherit",
                  color: "var(--neutral-500)",
                }}
              >
                <svg width="10" height="10" viewBox="0 0 12.5 12.5" aria-hidden="true">
                  <path
                    fill="currentColor"
                    d="M5.421 6.246 0 11.667l.825.825 5.421-5.421 5.421 5.421.825-.825-5.421-5.421L12.492.825 11.667 0 6.246 5.421.825 0 0 .825z"
                  />
                </svg>
              </button>
            )}
          </div>
        );
      })}
    </div>
  );
};
