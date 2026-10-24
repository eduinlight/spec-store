import {
  type KeyboardEvent,
  type ReactNode,
  useCallback,
  useEffect,
  useId,
  useRef,
  useState,
} from "react";
import { Checkbox } from "../taptap/Checkbox";
import { Icon } from "../taptap/Icon";
import { Tag } from "../taptap/Tag";
import { useOutsideDismiss } from "./use-outside-dismiss";

export interface FacetedFilterOption {
  value: string;
  label: string;
  count?: number;
  mono?: boolean;
}

export interface FacetedFilterProps {
  label: string;
  options: ReadonlyArray<FacetedFilterOption>;
  value?: ReadonlyArray<string>;
  defaultValue?: ReadonlyArray<string>;
  onChange?: (value: string[]) => void;
  searchPlaceholder?: string;
  emptyText?: string;
  clearLabel?: string;
  maxInlineTags?: number;
}

const NONE: ReadonlyArray<string> = [];

interface OptionRowProps {
  children: ReactNode;
}

function OptionRow({ children }: OptionRowProps) {
  const [hover, setHover] = useState(false);
  return (
    <div
      role="none"
      onMouseEnter={() => setHover(true)}
      onMouseLeave={() => setHover(false)}
      style={{
        borderRadius: 2,
        background: hover ? "var(--neutral-50)" : "transparent",
      }}
    >
      {children}
    </div>
  );
}

export function FacetedFilter({
  label,
  options,
  value,
  defaultValue = NONE,
  onChange,
  searchPlaceholder,
  emptyText = "No results found.",
  clearLabel = "Clear filters",
  maxInlineTags = 2,
}: FacetedFilterProps) {
  const [open, setOpen] = useState(false);
  const [query, setQuery] = useState("");
  const [inner, setInner] = useState<ReadonlyArray<string>>(defaultValue);
  const [hover, setHover] = useState(false);
  const [clearHover, setClearHover] = useState(false);
  const containerRef = useRef<HTMLDivElement>(null);
  const triggerRef = useRef<HTMLButtonElement>(null);
  const searchRef = useRef<HTMLInputElement>(null);
  const panelId = useId();
  const selected = value ?? inner;
  const visible = options.filter((option) =>
    option.label.toLowerCase().includes(query.trim().toLowerCase()),
  );
  const labelOf = (key: string) => options.find((option) => option.value === key)?.label ?? key;

  const close = useCallback(() => {
    setOpen(false);
    setQuery("");
  }, []);

  useOutsideDismiss(containerRef, open, close);

  useEffect(() => {
    if (open) {
      searchRef.current?.focus();
    }
  }, [open]);

  const commit = (next: string[]) => {
    if (value === undefined) {
      setInner(next);
    }
    onChange?.(next);
  };

  const toggle = (key: string) =>
    commit(selected.includes(key) ? selected.filter((item) => item !== key) : [...selected, key]);

  const onPanelKeyDown = (event: KeyboardEvent<HTMLDivElement>) => {
    if (event.key === "Escape") {
      event.preventDefault();
      close();
      triggerRef.current?.focus();
    }
  };

  const triggerName = selected.length > 0 ? `${label}, ${selected.length} selected` : label;

  return (
    <div ref={containerRef} style={{ position: "relative" }}>
      <button
        ref={triggerRef}
        type="button"
        aria-haspopup="dialog"
        aria-expanded={open}
        aria-controls={open ? panelId : undefined}
        aria-label={triggerName}
        onClick={() => (open ? close() : setOpen(true))}
        onMouseEnter={() => setHover(true)}
        onMouseLeave={() => setHover(false)}
        style={{
          height: 36,
          display: "flex",
          alignItems: "center",
          gap: 6,
          padding: "0 10px",
          borderRadius: 4,
          border: "1px dashed var(--neutral-400)",
          font: "400 14px/22px var(--font-sans)",
          color: "var(--neutral-600)",
          cursor: "pointer",
          background: open || hover ? "var(--neutral-50)" : "#fff",
        }}
      >
        <Icon name="PlusSmall" size={16} />
        <span>{label}</span>
        {selected.length > 0 ? (
          <>
            <span
              aria-hidden="true"
              style={{ width: 1, height: 16, background: "var(--neutral-300)", margin: "0 2px" }}
            />
            {selected.length > maxInlineTags ? (
              <Tag type="primary">{selected.length} selected</Tag>
            ) : (
              <span style={{ display: "flex", gap: 4 }}>
                {selected.map((key) => (
                  <Tag key={key} type="primary">
                    {labelOf(key)}
                  </Tag>
                ))}
              </span>
            )}
          </>
        ) : null}
      </button>
      {open ? (
        <div
          id={panelId}
          role="dialog"
          aria-label={label}
          onKeyDown={onPanelKeyDown}
          style={{
            position: "absolute",
            top: 40,
            left: 0,
            zIndex: 20,
            width: 240,
            background: "#fff",
            borderRadius: 4,
            boxShadow: "var(--elevation-2)",
            overflow: "hidden",
          }}
        >
          <div
            style={{
              display: "flex",
              alignItems: "center",
              gap: 8,
              padding: "0 10px",
              height: 40,
              borderBottom: "1px solid var(--neutral-200)",
              color: "var(--neutral-500)",
            }}
          >
            <Icon name="Search" size={16} />
            <input
              ref={searchRef}
              value={query}
              onChange={(event) => setQuery(event.target.value)}
              placeholder={searchPlaceholder ?? label}
              aria-label={searchPlaceholder ?? `Search ${label}`}
              style={{
                flex: 1,
                minWidth: 0,
                border: "none",
                outline: "none",
                background: "transparent",
                font: "400 14px/22px var(--font-sans)",
                color: "var(--neutral-700)",
              }}
            />
          </div>
          <div style={{ padding: 4, maxHeight: 240, overflowY: "auto" }}>
            {visible.map((option) => (
              <OptionRow key={option.value}>
                <Checkbox
                  checked={selected.includes(option.value)}
                  onChange={() => toggle(option.value)}
                  style={{
                    display: "flex",
                    width: "100%",
                    boxSizing: "border-box",
                    height: 36,
                    padding: "0 8px",
                    gap: 10,
                    color: "var(--neutral-700)",
                  }}
                  label={
                    <>
                      <span
                        style={{
                          flex: 1,
                          fontFamily: option.mono ? "var(--font-mono)" : "var(--font-sans)",
                        }}
                      >
                        {option.label}
                      </span>
                      {option.count !== undefined ? (
                        <span
                          style={{
                            font: "400 12px/18px var(--font-numeric)",
                            color: "var(--neutral-500)",
                          }}
                        >
                          {option.count}
                        </span>
                      ) : null}
                    </>
                  }
                />
              </OptionRow>
            ))}
            {visible.length === 0 ? (
              <div
                style={{
                  padding: "16px 8px",
                  textAlign: "center",
                  font: "400 13px/20px var(--font-sans)",
                  color: "var(--neutral-500)",
                }}
              >
                {emptyText}
              </div>
            ) : null}
          </div>
          {selected.length > 0 ? (
            <button
              type="button"
              onClick={() => commit([])}
              onMouseEnter={() => setClearHover(true)}
              onMouseLeave={() => setClearHover(false)}
              style={{
                width: "100%",
                height: 36,
                border: "none",
                borderTop: "1px solid var(--neutral-200)",
                display: "flex",
                alignItems: "center",
                justifyContent: "center",
                font: "400 13px/20px var(--font-sans)",
                color: "var(--neutral-600)",
                cursor: "pointer",
                background: clearHover ? "var(--neutral-50)" : "#fff",
              }}
            >
              {clearLabel}
            </button>
          ) : null}
        </div>
      ) : null}
    </div>
  );
}
