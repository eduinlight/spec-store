import {
  type CSSProperties,
  type InputHTMLAttributes,
  type ReactNode,
  useEffect,
  useRef,
  useState,
} from "react";

export interface CheckboxProps
  extends Omit<
    InputHTMLAttributes<HTMLInputElement>,
    "type" | "onChange" | "style" | "checked" | "defaultChecked"
  > {
  checked?: boolean;
  defaultChecked?: boolean;
  indeterminate?: boolean;
  disabled?: boolean;
  label?: ReactNode;
  onChange?: (checked: boolean) => void;
  style?: CSSProperties;
}

const HIDDEN_INPUT_STYLE: CSSProperties = {
  position: "absolute",
  width: 16,
  height: 16,
  margin: 0,
  opacity: 0,
  cursor: "inherit",
};

export function Checkbox({
  checked: checkedProp,
  defaultChecked = false,
  indeterminate = false,
  disabled = false,
  label,
  onChange,
  style,
  onFocus,
  onBlur,
  ...rest
}: CheckboxProps) {
  const [inner, setInner] = useState(defaultChecked);
  const [hover, setHover] = useState(false);
  const [focus, setFocus] = useState(false);
  const inputRef = useRef<HTMLInputElement>(null);
  const checked = checkedProp ?? inner;
  const on = checked || indeterminate;

  useEffect(() => {
    if (inputRef.current) {
      inputRef.current.indeterminate = indeterminate;
    }
  }, [indeterminate]);

  const background = disabled ? "var(--neutral-100)" : on ? "var(--primary-600)" : "#fff";
  const border =
    disabled || !on
      ? hover && !disabled
        ? "var(--primary-500)"
        : "var(--neutral-300)"
      : "transparent";
  const mark = disabled ? "var(--neutral-400)" : "#fff";
  const ring = focus && !disabled ? ", var(--focus-ring)" : "";

  return (
    <label
      onMouseEnter={() => setHover(true)}
      onMouseLeave={() => setHover(false)}
      style={{
        display: "inline-flex",
        alignItems: "center",
        gap: 8,
        cursor: disabled ? "not-allowed" : "pointer",
        font: "400 14px/22px var(--font-sans)",
        color: disabled ? "var(--neutral-400)" : "var(--neutral-600)",
        ...style,
      }}
    >
      <span
        style={{
          position: "relative",
          width: 16,
          height: 16,
          flexShrink: 0,
          borderRadius: 2,
          background,
          boxShadow: `inset 0 0 0 1px ${border}${ring}`,
          display: "inline-flex",
          alignItems: "center",
          justifyContent: "center",
          transition: "background .15s",
        }}
      >
        <input
          {...rest}
          ref={inputRef}
          type="checkbox"
          checked={checked}
          disabled={disabled}
          onChange={(event) => {
            if (checkedProp === undefined) {
              setInner(event.target.checked);
            }
            onChange?.(event.target.checked);
          }}
          onFocus={(event) => {
            setFocus(true);
            onFocus?.(event);
          }}
          onBlur={(event) => {
            setFocus(false);
            onBlur?.(event);
          }}
          style={HIDDEN_INPUT_STYLE}
        />
        {checked && !indeterminate && (
          <svg aria-hidden="true" width="10" height="10" viewBox="0 0 12 12">
            <path d="M2.5 6.2 5 8.5l4.5-5" fill="none" stroke={mark} strokeWidth="1.5" />
          </svg>
        )}
        {indeterminate && <span style={{ width: 8, height: 2, background: mark }} />}
      </span>
      {label}
    </label>
  );
}
