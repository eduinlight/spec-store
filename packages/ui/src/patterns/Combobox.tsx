import {
  type CSSProperties,
  type KeyboardEvent,
  useCallback,
  useEffect,
  useId,
  useRef,
  useState,
} from "react";
import { Icon, type IconName } from "../taptap/Icon";
import { useOutsideDismiss } from "./use-outside-dismiss";

export interface ComboboxItem {
  value: string;
  label: string;
  initial?: string;
  description?: string;
  meta?: string;
}

export interface ComboboxAction {
  key: string;
  label: string;
  icon: IconName;
  onSelect: () => void;
}

export interface ComboboxProps {
  items: ReadonlyArray<ComboboxItem>;
  value?: string;
  defaultValue?: string;
  onChange?: (value: string) => void;
  groupLabel: string;
  searchPlaceholder?: string;
  emptyText?: string;
  actions?: ReadonlyArray<ComboboxAction>;
  width?: number;
  mono?: boolean;
  showMetaInTrigger?: boolean;
  filled?: boolean;
  placeholder?: string;
  "aria-label"?: string;
}

const initialOf = (item: ComboboxItem) => item.initial ?? item.label.slice(0, 1).toUpperCase();

const matches = (item: ComboboxItem, query: string) =>
  `${item.label} ${item.description ?? ""}`.toLowerCase().includes(query.trim().toLowerCase());

interface ActionRowProps {
  action: ComboboxAction;
  onPick: (action: ComboboxAction) => void;
}

function ActionRow({ action, onPick }: ActionRowProps) {
  const [hover, setHover] = useState(false);
  return (
    <button
      type="button"
      onClick={() => onPick(action)}
      onMouseEnter={() => setHover(true)}
      onMouseLeave={() => setHover(false)}
      style={{
        display: "flex",
        alignItems: "center",
        gap: 10,
        width: "100%",
        height: 36,
        padding: "0 8px",
        border: "none",
        borderRadius: 2,
        background: hover ? "var(--neutral-50)" : "transparent",
        cursor: "pointer",
        font: "400 14px/22px var(--font-sans)",
        color: "var(--neutral-600)",
        textAlign: "left",
      }}
    >
      <Icon name={action.icon} size={16} />
      {action.label}
    </button>
  );
}

export function Combobox({
  items,
  value,
  defaultValue,
  onChange,
  groupLabel,
  searchPlaceholder = "Search…",
  emptyText = "Nothing found.",
  actions = [],
  width,
  mono = false,
  showMetaInTrigger = false,
  filled = false,
  placeholder = "Select",
  "aria-label": ariaLabel,
}: ComboboxProps) {
  const [open, setOpen] = useState(false);
  const [query, setQuery] = useState("");
  const [active, setActive] = useState(0);
  const [inner, setInner] = useState(defaultValue);
  const [hover, setHover] = useState(false);
  const containerRef = useRef<HTMLDivElement>(null);
  const triggerRef = useRef<HTMLButtonElement>(null);
  const listRef = useRef<HTMLDivElement>(null);
  const searchRef = useRef<HTMLInputElement>(null);
  const listId = useId();
  const groupId = useId();
  const current = value ?? inner;
  const currentItem = items.find((item) => item.value === current);
  const visible = items.filter((item) => matches(item, query));
  const activeIndex = Math.min(active, visible.length - 1);
  const panelWidth = width ?? (mono ? 340 : 280);

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

  useEffect(() => {
    if (open && activeIndex >= 0) {
      listRef.current
        ?.querySelector(`[data-index="${activeIndex}"]`)
        ?.scrollIntoView?.({ block: "nearest" });
    }
  }, [open, activeIndex]);

  const openPanel = () => {
    const index = items.findIndex((item) => item.value === current);
    setActive(Math.max(index, 0));
    setOpen(true);
  };

  const closeAndFocus = () => {
    close();
    triggerRef.current?.focus();
  };

  const pick = (item: ComboboxItem) => {
    if (value === undefined) {
      setInner(item.value);
    }
    onChange?.(item.value);
    closeAndFocus();
  };

  const pickAction = (action: ComboboxAction) => {
    close();
    action.onSelect();
  };

  const onSearchKeyDown = (event: KeyboardEvent<HTMLInputElement>) => {
    switch (event.key) {
      case "ArrowDown":
        event.preventDefault();
        setActive(Math.min(activeIndex + 1, visible.length - 1));
        break;
      case "ArrowUp":
        event.preventDefault();
        setActive(Math.max(activeIndex - 1, 0));
        break;
      case "Home":
        event.preventDefault();
        setActive(0);
        break;
      case "End":
        event.preventDefault();
        setActive(Math.max(visible.length - 1, 0));
        break;
      case "Enter": {
        event.preventDefault();
        const item = visible[activeIndex];
        if (item) {
          pick(item);
        }
        break;
      }
      case "Escape":
        event.preventDefault();
        closeAndFocus();
        break;
      case "Tab":
        close();
        break;
    }
  };

  const onTriggerKeyDown = (event: KeyboardEvent<HTMLButtonElement>) => {
    if (event.key === "ArrowDown" && !open) {
      event.preventDefault();
      openPanel();
    }
    if (event.key === "Escape" && open) {
      event.preventDefault();
      closeAndFocus();
    }
  };

  const swatch: CSSProperties = mono
    ? { background: "var(--neutral-700)", color: "#fff", font: "500 11px/20px var(--font-mono)" }
    : { background: "#fff", color: "var(--primary-700)", font: "500 11px/20px var(--font-sans)" };

  const triggerBackground = open
    ? "rgba(255,255,255,.24)"
    : hover
      ? "rgba(255,255,255,.16)"
      : filled
        ? "rgba(255,255,255,.16)"
        : "transparent";

  return (
    <div ref={containerRef} style={{ position: "relative" }}>
      <button
        ref={triggerRef}
        type="button"
        aria-haspopup="listbox"
        aria-expanded={open}
        aria-controls={open ? listId : undefined}
        aria-label={ariaLabel}
        onClick={() => (open ? close() : openPanel())}
        onKeyDown={onTriggerKeyDown}
        onMouseEnter={() => setHover(true)}
        onMouseLeave={() => setHover(false)}
        style={{
          display: "flex",
          alignItems: "center",
          gap: 8,
          height: 36,
          padding: "0 8px 0 10px",
          border: "none",
          borderRadius: 4,
          cursor: "pointer",
          color: "inherit",
          font: `500 14px/22px ${mono ? "var(--font-mono)" : "var(--font-sans)"}`,
          background: triggerBackground,
        }}
      >
        {currentItem ? (
          <>
            <span
              aria-hidden="true"
              style={{
                width: 20,
                height: 20,
                borderRadius: 2,
                textAlign: "center",
                ...swatch,
              }}
            >
              {initialOf(currentItem)}
            </span>
            <span style={{ whiteSpace: "nowrap" }}>{currentItem.label}</span>
            {showMetaInTrigger && currentItem.meta ? (
              <span
                style={{
                  padding: "0 6px",
                  borderRadius: 2,
                  background: "#fff",
                  color: "var(--primary-700)",
                  font: "500 11px/18px var(--font-sans)",
                  whiteSpace: "nowrap",
                }}
              >
                {currentItem.meta}
              </span>
            ) : null}
          </>
        ) : (
          <span style={{ whiteSpace: "nowrap", opacity: 0.8 }}>{placeholder}</span>
        )}
        <span style={{ display: "flex", opacity: 0.8 }}>
          <Icon name="Sort" size={16} />
        </span>
      </button>
      {open ? (
        <div
          style={{
            position: "absolute",
            top: 44,
            left: 0,
            zIndex: 20,
            width: panelWidth,
            background: "#fff",
            borderRadius: 4,
            boxShadow: "var(--elevation-2)",
            color: "var(--neutral-700)",
            overflow: "hidden",
          }}
        >
          <div
            style={{
              display: "flex",
              alignItems: "center",
              gap: 8,
              height: 40,
              padding: "0 12px",
              borderBottom: "1px solid var(--neutral-200)",
              color: "var(--neutral-500)",
            }}
          >
            <Icon name="Search" size={16} />
            <input
              ref={searchRef}
              value={query}
              onChange={(event) => {
                setQuery(event.target.value);
                setActive(0);
              }}
              onKeyDown={onSearchKeyDown}
              placeholder={searchPlaceholder}
              aria-label={searchPlaceholder}
              aria-controls={listId}
              aria-activedescendant={activeIndex >= 0 ? `${listId}-${activeIndex}` : undefined}
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
          <div ref={listRef} style={{ padding: 4, maxHeight: mono ? 300 : 280, overflowY: "auto" }}>
            <div
              id={groupId}
              style={{
                padding: "6px 8px 4px",
                font: "400 12px/18px var(--font-sans)",
                color: "var(--neutral-500)",
              }}
            >
              {groupLabel}
            </div>
            <div id={listId} role="listbox" aria-labelledby={groupId}>
              {visible.map((item, index) => {
                const selected = item.value === current;
                const highlighted = index === activeIndex;
                return (
                  // biome-ignore lint/a11y/useKeyWithClickEvents: keyboard selection is handled by the search field through aria-activedescendant
                  <div
                    key={item.value}
                    id={`${listId}-${index}`}
                    data-index={index}
                    role="option"
                    tabIndex={-1}
                    aria-selected={selected}
                    onClick={() => pick(item)}
                    onMouseDown={(event) => event.preventDefault()}
                    onMouseEnter={() => setActive(index)}
                    style={{
                      display: "flex",
                      alignItems: "center",
                      gap: 10,
                      height: item.description ? undefined : 40,
                      padding: item.description ? "6px 8px" : "0 8px",
                      borderRadius: 2,
                      cursor: "pointer",
                      background: highlighted
                        ? "var(--neutral-50)"
                        : selected
                          ? "var(--primary-50)"
                          : "transparent",
                    }}
                  >
                    <span
                      aria-hidden="true"
                      style={{
                        width: 24,
                        height: 24,
                        borderRadius: 2,
                        textAlign: "center",
                        background: mono ? "var(--neutral-700)" : "var(--primary-200)",
                        color: mono ? "#fff" : "var(--primary-700)",
                        font: `500 12px/24px ${mono ? "var(--font-mono)" : "var(--font-sans)"}`,
                      }}
                    >
                      {initialOf(item)}
                    </span>
                    <span
                      style={{
                        flex: 1,
                        minWidth: 0,
                        display: "flex",
                        flexDirection: "column",
                      }}
                    >
                      <span
                        style={{
                          font: mono
                            ? "500 14px/20px var(--font-mono)"
                            : "400 14px/22px var(--font-sans)",
                          color: selected ? "var(--primary-700)" : "var(--neutral-700)",
                        }}
                      >
                        {item.label}
                      </span>
                      {item.description ? (
                        <span
                          style={{
                            font: "400 12px/18px var(--font-sans)",
                            color: "var(--neutral-500)",
                          }}
                        >
                          {item.description}
                        </span>
                      ) : null}
                    </span>
                    {item.meta ? (
                      <span
                        style={{
                          font: "400 12px/18px var(--font-sans)",
                          color: "var(--neutral-500)",
                        }}
                      >
                        {item.meta}
                      </span>
                    ) : null}
                    <span style={{ width: 16, display: "flex", color: "var(--primary-600)" }}>
                      {selected ? <Icon name="Check" size={16} /> : null}
                    </span>
                  </div>
                );
              })}
            </div>
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
          {actions.length > 0 ? (
            <div
              style={{
                borderTop: "1px solid var(--neutral-200)",
                padding: 4,
                display: "flex",
                flexDirection: "column",
              }}
            >
              {actions.map((action) => (
                <ActionRow key={action.key} action={action} onPick={pickAction} />
              ))}
            </div>
          ) : null}
        </div>
      ) : null}
    </div>
  );
}
