import type { ButtonHTMLAttributes, CSSProperties } from "react";
import { useState } from "react";
import { useInteraction } from "./interaction";

export interface ToggleProps
  extends Omit<
    ButtonHTMLAttributes<HTMLButtonElement>,
    "onChange" | "style" | "type" | "role" | "children"
  > {
  checked?: boolean;
  defaultChecked?: boolean;
  disabled?: boolean;
  showText?: boolean;
  onChange?: (checked: boolean) => void;
  style?: CSSProperties;
}

export function Toggle({
  checked: checkedProp,
  defaultChecked = false,
  disabled = false,
  showText = false,
  onChange,
  style,
  onFocus,
  onBlur,
  ...rest
}: ToggleProps) {
  const [inner, setInner] = useState(defaultChecked);
  const [state, handlers] = useInteraction(disabled);
  const checked = checkedProp ?? inner;
  const background = checked
    ? disabled
      ? "var(--primary-400)"
      : "var(--primary-600)"
    : disabled
      ? "var(--neutral-200)"
      : "var(--neutral-400)";
  const width = showText ? (checked ? 45 : 50) : 36;
  const toggle = () => {
    if (checkedProp === undefined) {
      setInner(!checked);
    }
    onChange?.(!checked);
  };
  return (
    <button
      {...rest}
      type="button"
      role="switch"
      aria-checked={checked}
      disabled={disabled}
      onClick={toggle}
      onFocus={(event) => {
        handlers.onFocus?.(event);
        onFocus?.(event);
      }}
      onBlur={(event) => {
        handlers.onBlur?.(event);
        onBlur?.(event);
      }}
      style={{
        position: "relative",
        width,
        height: 20,
        padding: 2,
        border: "none",
        outline: "none",
        borderRadius: 20,
        background,
        boxShadow: state === "focus" ? "var(--focus-ring)" : "none",
        cursor: disabled ? "not-allowed" : "pointer",
        display: "inline-flex",
        alignItems: "center",
        flexDirection: checked ? "row" : "row-reverse",
        boxSizing: "border-box",
        transition: "background .2s",
        ...style,
      }}
    >
      {showText && (
        <span
          style={{
            flex: 1,
            textAlign: "center",
            font: "500 12px/18px var(--font-sans)",
            color: "#fff",
          }}
        >
          {checked ? "ON" : "OFF"}
        </span>
      )}
      <span
        style={{
          width: 16,
          height: 16,
          borderRadius: 16,
          background: "#fff",
          marginLeft: checked && !showText ? "auto" : 0,
          marginRight: !checked && !showText ? "auto" : 0,
          flexShrink: 0,
          boxShadow: "0 1px 2px rgba(0,0,0,0.12)",
        }}
      />
    </button>
  );
}
