import { type CSSProperties, type KeyboardEvent, useEffect, useId, useRef, useState } from "react";
import { DropdownMenu, type DropdownMenuItem, itemKey } from "./DropdownMenu";
import { Chevron, type FieldSize, fieldShell } from "./field";

export type SelectValue = string | number;
export type SelectOption = DropdownMenuItem;

interface SelectBaseProps {
  options?: SelectOption[];
  placeholder?: string;
  size?: FieldSize;
  disabled?: boolean;
  error?: boolean;
  width?: number | string;
  open?: boolean;
  style?: CSSProperties;
  id?: string;
  "aria-label"?: string;
  "aria-labelledby"?: string;
}

export interface SingleSelectProps extends SelectBaseProps {
  multiple?: false;
  value?: SelectValue;
  defaultValue?: SelectValue;
  onChange?: (value: SelectValue) => void;
}

export interface MultipleSelectProps extends SelectBaseProps {
  multiple: true;
  value?: SelectValue[];
  defaultValue?: SelectValue[];
  onChange?: (value: SelectValue[]) => void;
}

export type SelectProps = SingleSelectProps | MultipleSelectProps;

const MIN_HEIGHT: Record<FieldSize, number> = { large: 40, medium: 36, small: 24 };
const NO_SELECTION: SelectValue[] = [];

export function Select(props: SelectProps) {
  const {
    options = [],
    placeholder = "Select",
    size = "medium",
    disabled = false,
    error = false,
    width = 200,
    open: openProp,
    style,
    id,
    "aria-label": ariaLabel,
    "aria-labelledby": ariaLabelledBy,
  } = props;
  const multiple = props.multiple === true;
  const [open, setOpen] = useState(false);
  const [hover, setHover] = useState(false);
  const [focus, setFocus] = useState(false);
  const [active, setActive] = useState(-1);
  const [inner, setInner] = useState<SelectValue | SelectValue[] | undefined>(
    props.defaultValue ?? (props.multiple ? [] : undefined),
  );
  const containerRef = useRef<HTMLDivElement>(null);
  const listId = useId();
  const current = props.value ?? inner;
  const selectedList = Array.isArray(current) ? current : NO_SELECTION;
  const selectedSingle = Array.isArray(current) ? undefined : current;
  const isOpen = openProp ?? open;
  const empty = multiple ? selectedList.length === 0 : selectedSingle == null;
  const enabledIndexes = options.flatMap((option, index) =>
    option.divider || option.disabled ? [] : [index],
  );

  useEffect(() => {
    const close = (event: MouseEvent) => {
      if (containerRef.current && !containerRef.current.contains(event.target as Node)) {
        setOpen(false);
      }
    };
    document.addEventListener("mousedown", close);
    return () => document.removeEventListener("mousedown", close);
  }, []);

  const labelOf = (key: SelectValue) =>
    options.find((option) => !option.divider && itemKey(option) === key)?.label;

  const pick = (key: SelectValue) => {
    if (props.multiple) {
      const next = selectedList.includes(key)
        ? selectedList.filter((item) => item !== key)
        : [...selectedList, key];
      if (props.value === undefined) {
        setInner(next);
      }
      props.onChange?.(next);
      return;
    }
    setOpen(false);
    if (props.value === undefined) {
      setInner(key);
    }
    props.onChange?.(key);
  };

  const openMenu = () => {
    const selectedIndex = options.findIndex(
      (option) => !option.divider && !option.disabled && itemKey(option) === selectedSingle,
    );
    setActive(selectedIndex >= 0 && !multiple ? selectedIndex : (enabledIndexes[0] ?? -1));
    setOpen(true);
  };

  const move = (step: number) => {
    const position = enabledIndexes.indexOf(active);
    const next = Math.min(
      Math.max(position < 0 ? (step > 0 ? 0 : enabledIndexes.length - 1) : position + step, 0),
      enabledIndexes.length - 1,
    );
    setActive(enabledIndexes[next] ?? -1);
  };

  const onKeyDown = (event: KeyboardEvent<HTMLDivElement>) => {
    if (disabled) {
      return;
    }
    if (!isOpen) {
      if (["Enter", " ", "ArrowDown", "ArrowUp"].includes(event.key)) {
        event.preventDefault();
        openMenu();
      }
      return;
    }
    switch (event.key) {
      case "ArrowDown":
        event.preventDefault();
        move(1);
        break;
      case "ArrowUp":
        event.preventDefault();
        move(-1);
        break;
      case "Home":
        event.preventDefault();
        setActive(enabledIndexes[0] ?? -1);
        break;
      case "End":
        event.preventDefault();
        setActive(enabledIndexes[enabledIndexes.length - 1] ?? -1);
        break;
      case "Enter":
      case " ": {
        event.preventDefault();
        const option = options[active];
        if (option && !option.divider && !option.disabled) {
          pick(itemKey(option));
        }
        break;
      }
      case "Escape":
        event.preventDefault();
        setOpen(false);
        break;
      case "Tab":
        setOpen(false);
        break;
    }
  };

  const toggle = () => {
    if (disabled) {
      return;
    }
    if (isOpen) {
      setOpen(false);
    } else {
      openMenu();
    }
  };

  const triggerStyle: CSSProperties = {
    ...fieldShell({
      size,
      state: isOpen || focus ? "focus" : hover ? "hover" : "default",
      disabled,
      error,
    }),
    cursor: disabled ? "not-allowed" : "pointer",
    height: multiple ? "auto" : MIN_HEIGHT[size],
    minHeight: MIN_HEIGHT[size],
    flexWrap: multiple ? "wrap" : "nowrap",
    outline: "none",
    ...(multiple && !empty ? { padding: "3px 12px 3px 4px" } : {}),
  };

  return (
    <div ref={containerRef} style={{ position: "relative", width, ...style }}>
      <div
        id={id}
        role="combobox"
        tabIndex={disabled ? -1 : 0}
        aria-expanded={isOpen}
        aria-haspopup="listbox"
        aria-controls={isOpen ? listId : undefined}
        aria-activedescendant={isOpen && active >= 0 ? `${listId}-option-${active}` : undefined}
        aria-disabled={disabled}
        aria-invalid={error || undefined}
        aria-label={ariaLabel}
        aria-labelledby={ariaLabelledBy}
        onClick={toggle}
        onKeyDown={onKeyDown}
        onFocus={() => setFocus(true)}
        onBlur={() => setFocus(false)}
        onMouseEnter={() => setHover(true)}
        onMouseLeave={() => setHover(false)}
        style={triggerStyle}
      >
        <span
          style={{
            flex: 1,
            minWidth: 0,
            display: "flex",
            flexWrap: "wrap",
            gap: 4,
            color: empty ? "var(--neutral-400)" : undefined,
            whiteSpace: "nowrap",
            overflow: "hidden",
            textOverflow: "ellipsis",
          }}
        >
          {empty
            ? placeholder
            : multiple
              ? selectedList.map((key) => (
                  <span
                    key={key}
                    style={{
                      display: "inline-flex",
                      alignItems: "center",
                      gap: 4,
                      height: 24,
                      padding: "0 4px 0 8px",
                      borderRadius: 2,
                      background: "var(--neutral-100)",
                      font: "400 12px/18px var(--font-sans)",
                      color: "var(--neutral-600)",
                    }}
                  >
                    {labelOf(key)}
                    <button
                      type="button"
                      tabIndex={-1}
                      aria-label={`Remove ${labelOf(key) ?? key}`}
                      onClick={(event) => {
                        event.stopPropagation();
                        if (!disabled) {
                          pick(key);
                        }
                      }}
                      style={{
                        display: "inline-flex",
                        padding: 0,
                        border: "none",
                        background: "transparent",
                        color: "var(--neutral-500)",
                        cursor: "pointer",
                      }}
                    >
                      <svg aria-hidden="true" width="8" height="8" viewBox="0 0 12.5 12.5">
                        <path
                          fill="currentColor"
                          d="M5.421 6.246 0 11.667l.825.825 5.421-5.421 5.421 5.421.825-.825-5.421-5.421L12.492.825 11.667 0 6.246 5.421.825 0 0 .825z"
                        />
                      </svg>
                    </button>
                  </span>
                ))
              : selectedSingle != null && labelOf(selectedSingle)}
        </span>
        <span style={{ color: "var(--neutral-500)", display: "inline-flex" }}>
          <Chevron up={isOpen} />
        </span>
      </div>
      {isOpen && (
        <DropdownMenu
          id={listId}
          idPrefix={listId}
          items={options}
          value={current}
          multiple={multiple}
          onSelect={pick}
          size={size}
          activeIndex={active}
          style={{
            position: "absolute",
            top: "100%",
            left: 0,
            right: 0,
            marginTop: 4,
            zIndex: 20,
            animation: "tt-fade-in .12s",
          }}
        />
      )}
    </div>
  );
}
