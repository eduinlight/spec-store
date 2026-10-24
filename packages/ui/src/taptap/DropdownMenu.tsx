import { type CSSProperties, type ReactNode, useState } from "react";
import type { FieldSize } from "./field";

export type DropdownMenuKey = string | number;

export interface DropdownMenuItem {
  label?: string;
  value?: DropdownMenuKey;
  disabled?: boolean;
  divider?: boolean;
  avatar?: string;
  icon?: ReactNode;
  extra?: ReactNode;
}

export interface DropdownMenuProps {
  items?: DropdownMenuItem[];
  value?: DropdownMenuKey | DropdownMenuKey[];
  multiple?: boolean;
  onSelect?: (key: DropdownMenuKey) => void;
  size?: FieldSize;
  style?: CSSProperties;
  id?: string;
  idPrefix?: string;
  activeIndex?: number;
  "aria-label"?: string;
}

const PADDING: Record<FieldSize, string> = {
  large: "6px 8px",
  medium: "5px 8px",
  small: "1px 8px",
};
const FONT_SIZE: Record<FieldSize, number> = { large: 16, medium: 14, small: 12 };
const LINE_HEIGHT: Record<FieldSize, string> = { large: "24px", medium: "22px", small: "18px" };

export const itemKey = (item: DropdownMenuItem): DropdownMenuKey => item.value ?? item.label ?? "";

export function DropdownMenu({
  items = [],
  value,
  multiple = false,
  onSelect,
  size = "medium",
  style,
  id,
  idPrefix,
  activeIndex,
  "aria-label": ariaLabel,
}: DropdownMenuProps) {
  const [hover, setHover] = useState<number | null>(null);
  const entries = items.map((item, index) => ({
    item,
    index,
    rowKey: item.divider ? `divider-${index}` : String(itemKey(item)),
  }));
  const isSelected = (key: DropdownMenuKey) =>
    multiple ? Array.isArray(value) && value.includes(key) : key === value;
  return (
    <div
      id={id}
      role="listbox"
      aria-multiselectable={multiple || undefined}
      aria-label={ariaLabel}
      style={{
        background: "#fff",
        borderRadius: 4,
        padding: "4px",
        boxShadow: "var(--elevation-2)",
        display: "flex",
        flexDirection: "column",
        gap: 2,
        minWidth: 160,
        ...style,
      }}
    >
      {entries.map(({ item, index, rowKey }) => {
        if (item.divider) {
          return (
            <hr
              key={rowKey}
              style={{
                height: 1,
                border: "none",
                background: "var(--neutral-200)",
                margin: "2px 0",
              }}
            />
          );
        }
        const key = itemKey(item);
        const selected = isSelected(key);
        const highlighted = (hover === index || activeIndex === index) && !item.disabled;
        return (
          <div
            key={rowKey}
            id={idPrefix ? `${idPrefix}-option-${index}` : undefined}
            role="option"
            tabIndex={-1}
            aria-selected={selected}
            aria-disabled={item.disabled || undefined}
            onMouseEnter={() => setHover(index)}
            onMouseLeave={() => setHover(null)}
            onMouseDown={(event) => event.preventDefault()}
            onClick={() => !item.disabled && onSelect?.(key)}
            onKeyDown={(event) => {
              if ((event.key === "Enter" || event.key === " ") && !item.disabled) {
                onSelect?.(key);
              }
            }}
            style={{
              display: "flex",
              alignItems: "center",
              gap: 8,
              padding: PADDING[size],
              borderRadius: 2,
              font: `400 ${FONT_SIZE[size]}px/${LINE_HEIGHT[size]} var(--font-sans)`,
              background: selected
                ? "var(--primary-100)"
                : highlighted
                  ? "var(--neutral-50)"
                  : "transparent",
              color: item.disabled
                ? "var(--neutral-400)"
                : selected
                  ? "var(--primary-600)"
                  : "var(--neutral-600)",
              cursor: item.disabled ? "not-allowed" : "pointer",
            }}
          >
            {multiple && (
              <span
                style={{
                  width: 16,
                  height: 16,
                  borderRadius: 2,
                  flexShrink: 0,
                  background: selected ? "var(--primary-600)" : "#fff",
                  boxShadow: selected ? "none" : "inset 0 0 0 1px var(--neutral-300)",
                  display: "inline-flex",
                  alignItems: "center",
                  justifyContent: "center",
                }}
              >
                {selected && (
                  <svg aria-hidden="true" width="10" height="10" viewBox="0 0 12 12">
                    <path d="M2.5 6.2 5 8.5l4.5-5" fill="none" stroke="#fff" strokeWidth="1.5" />
                  </svg>
                )}
              </span>
            )}
            {item.avatar && (
              <img
                src={item.avatar}
                alt=""
                style={{ width: 20, height: 20, borderRadius: "50%" }}
              />
            )}
            {item.icon}
            <span
              style={{
                flex: 1,
                whiteSpace: "nowrap",
                overflow: "hidden",
                textOverflow: "ellipsis",
              }}
            >
              {item.label}
            </span>
            {item.extra && <span style={{ color: "var(--neutral-400)" }}>{item.extra}</span>}
          </div>
        );
      })}
    </div>
  );
}
