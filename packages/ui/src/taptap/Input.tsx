import {
  type CSSProperties,
  type InputHTMLAttributes,
  type ReactNode,
  type Ref,
  useState,
} from "react";
import { type FieldSize, fieldShell, inputReset } from "./field";

export interface InputProps
  extends Omit<
    InputHTMLAttributes<HTMLInputElement>,
    "size" | "value" | "defaultValue" | "onChange" | "prefix" | "style"
  > {
  size?: FieldSize;
  value?: string;
  defaultValue?: string;
  placeholder?: string;
  disabled?: boolean;
  error?: boolean;
  prefix?: ReactNode;
  suffix?: ReactNode;
  allowClear?: boolean;
  onChange?: (value: string) => void;
  style?: CSSProperties;
  inputStyle?: CSSProperties;
  ref?: Ref<HTMLInputElement>;
}

const BARE_BUTTON_STYLE: CSSProperties = {
  display: "inline-flex",
  padding: 0,
  border: "none",
  background: "transparent",
  font: "inherit",
  cursor: "pointer",
};

const ADORNMENT_STYLE: CSSProperties = { display: "inline-flex", color: "var(--neutral-500)" };

export function Input({
  size = "medium",
  value,
  defaultValue,
  placeholder = "Text input",
  disabled = false,
  error = false,
  prefix,
  suffix,
  allowClear = false,
  onChange,
  style,
  inputStyle,
  ref,
  ...rest
}: InputProps) {
  const [focus, setFocus] = useState(false);
  const [hover, setHover] = useState(false);
  const [inner, setInner] = useState(defaultValue ?? "");
  const current = value ?? inner;
  const update = (next: string) => {
    if (value === undefined) {
      setInner(next);
    }
    onChange?.(next);
  };
  const state = focus ? "focus" : hover ? "hover" : "default";
  return (
    <label
      onMouseEnter={() => setHover(true)}
      onMouseLeave={() => setHover(false)}
      style={{ ...fieldShell({ size, state, disabled, error }), ...style }}
    >
      {prefix && <span style={ADORNMENT_STYLE}>{prefix}</span>}
      <input
        {...rest}
        ref={ref}
        disabled={disabled}
        value={current}
        placeholder={placeholder}
        onFocus={() => setFocus(true)}
        onBlur={() => setFocus(false)}
        onChange={(event) => update(event.target.value)}
        style={{ ...inputReset, ...inputStyle }}
        className="tt-input"
      />
      {allowClear && current && !disabled && (
        <button
          type="button"
          tabIndex={-1}
          aria-label="Clear"
          onMouseDown={(event) => {
            event.preventDefault();
            update("");
          }}
          style={{ ...BARE_BUTTON_STYLE, color: "var(--neutral-400)" }}
        >
          <svg aria-hidden="true" width="14" height="14" viewBox="0 0 16 16">
            <circle cx="8" cy="8" r="7" fill="currentColor" />
            <path d="M5.5 5.5l5 5m0-5-5 5" stroke="#fff" strokeWidth="1.2" fill="none" />
          </svg>
        </button>
      )}
      {suffix && <span style={ADORNMENT_STYLE}>{suffix}</span>}
      <style>
        {
          ".tt-input::placeholder{color:var(--neutral-400)}.tt-input:disabled::placeholder{color:var(--neutral-300)}"
        }
      </style>
    </label>
  );
}
