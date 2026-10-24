import type { ComponentPropsWithRef, FocusEvent, MouseEvent, ReactNode } from "react";
import { useInteraction } from "./interaction";

export type ButtonVariant = "primary" | "outline" | "ghost" | "link";
export type ButtonSize = "large" | "medium" | "small";
export type IconButtonVariant = "primary" | "outline" | "ghost";
export type IconButtonShape = "circle" | "square";

type ButtonState = "default" | "hover" | "pressed" | "focus" | "disabled";
type Palette = readonly [string, string | null, string];
type VariantPalette = Record<"default" | "hover" | "pressed" | "disabled", Palette>;

const SIZES: Record<
  ButtonSize,
  {
    height: number;
    padding: string;
    minWidth: number;
    fontSize: number;
    lineHeight: string;
    gap: number;
    icon: number;
  }
> = {
  large: {
    height: 40,
    padding: "8px 8px",
    minWidth: 90,
    fontSize: 16,
    lineHeight: "24px",
    gap: 4,
    icon: 16,
  },
  medium: {
    height: 36,
    padding: "7px 8px",
    minWidth: 82,
    fontSize: 14,
    lineHeight: "22px",
    gap: 4,
    icon: 16,
  },
  small: {
    height: 24,
    padding: "3px 4px",
    minWidth: 54,
    fontSize: 12,
    lineHeight: "18px",
    gap: 4,
    icon: 12,
  },
};

const VARIANTS: Record<ButtonVariant, VariantPalette> = {
  primary: {
    default: ["var(--primary-600)", null, "#fff"],
    hover: ["var(--primary-500)", null, "#fff"],
    pressed: ["var(--primary-700)", null, "#fff"],
    disabled: ["var(--primary-300)", null, "#fff"],
  },
  outline: {
    default: ["#fff", "var(--neutral-300)", "var(--neutral-600)"],
    hover: ["#fff", "var(--primary-500)", "var(--primary-500)"],
    pressed: ["#fff", "var(--primary-700)", "var(--primary-700)"],
    disabled: ["#fff", "var(--neutral-200)", "var(--neutral-400)"],
  },
  ghost: {
    default: ["transparent", null, "var(--neutral-600)"],
    hover: ["var(--neutral-100)", null, "var(--neutral-600)"],
    pressed: ["var(--neutral-200)", null, "var(--neutral-600)"],
    disabled: ["transparent", null, "var(--neutral-400)"],
  },
  link: {
    default: ["transparent", null, "var(--primary-600)"],
    hover: ["transparent", null, "var(--primary-500)"],
    pressed: ["transparent", null, "var(--primary-700)"],
    disabled: ["transparent", null, "var(--primary-300)"],
  },
};

const DANGER: Record<ButtonVariant, VariantPalette> = {
  primary: {
    default: ["var(--danger-600)", null, "#fff"],
    hover: ["var(--danger-500)", null, "#fff"],
    pressed: ["var(--danger-700)", null, "#fff"],
    disabled: ["var(--danger-300)", null, "#fff"],
  },
  outline: {
    default: ["#fff", "var(--danger-600)", "var(--danger-600)"],
    hover: ["#fff", "var(--danger-500)", "var(--danger-500)"],
    pressed: ["#fff", "var(--danger-700)", "var(--danger-700)"],
    disabled: ["#fff", "var(--danger-300)", "var(--danger-400)"],
  },
  ghost: {
    default: ["transparent", null, "var(--danger-600)"],
    hover: ["var(--danger-100)", null, "var(--danger-600)"],
    pressed: ["var(--danger-200)", null, "var(--danger-700)"],
    disabled: ["transparent", null, "var(--danger-400)"],
  },
  link: {
    default: ["transparent", null, "var(--danger-600)"],
    hover: ["transparent", null, "var(--danger-500)"],
    pressed: ["transparent", null, "var(--danger-700)"],
    disabled: ["transparent", null, "var(--danger-400)"],
  },
};

const ICON_BUTTON_SIZES: Record<ButtonSize, { box: number; icon: number }> = {
  large: { box: 40, icon: 20 },
  medium: { box: 36, icon: 16 },
  small: { box: 24, icon: 12 },
};

const ICON_BUTTON_VARIANTS: Record<IconButtonVariant, VariantPalette> = {
  primary: VARIANTS.primary,
  outline: VARIANTS.outline,
  ghost: VARIANTS.ghost,
};

const paletteState = (state: ButtonState): keyof VariantPalette =>
  state === "focus" ? "default" : state;

const shadow = (border: string | null, focused: boolean): string => {
  const parts = [border ? `inset 0 0 0 1px ${border}` : null, focused ? "var(--focus-ring)" : null];
  return parts.filter(Boolean).join(", ") || "none";
};

export interface ButtonProps extends ComponentPropsWithRef<"button"> {
  variant?: ButtonVariant;
  size?: ButtonSize;
  danger?: boolean;
  leftIcon?: ReactNode;
  rightIcon?: ReactNode;
  block?: boolean;
}

export function Button({
  variant = "primary",
  size = "medium",
  danger = false,
  disabled = false,
  leftIcon,
  rightIcon,
  block = false,
  children = "Text",
  style,
  type = "button",
  onMouseEnter,
  onMouseLeave,
  onMouseDown,
  onMouseUp,
  onFocus,
  onBlur,
  ...rest
}: ButtonProps) {
  const [state, handlers] = useInteraction(disabled);
  const s = SIZES[size] ?? SIZES.medium;
  const palette = (danger ? DANGER : VARIANTS)[variant] ?? VARIANTS.primary;
  const [bg, border, color] = palette[paletteState(state)];
  const isLink = variant === "link";
  return (
    <button
      type={type}
      disabled={disabled}
      {...rest}
      onMouseEnter={(event: MouseEvent<HTMLButtonElement>) => {
        handlers.onMouseEnter?.(event);
        onMouseEnter?.(event);
      }}
      onMouseLeave={(event: MouseEvent<HTMLButtonElement>) => {
        handlers.onMouseLeave?.(event);
        onMouseLeave?.(event);
      }}
      onMouseDown={(event: MouseEvent<HTMLButtonElement>) => {
        handlers.onMouseDown?.(event);
        onMouseDown?.(event);
      }}
      onMouseUp={(event: MouseEvent<HTMLButtonElement>) => {
        handlers.onMouseUp?.(event);
        onMouseUp?.(event);
      }}
      onFocus={(event: FocusEvent<HTMLButtonElement>) => {
        handlers.onFocus?.(event);
        onFocus?.(event);
      }}
      onBlur={(event: FocusEvent<HTMLButtonElement>) => {
        handlers.onBlur?.(event);
        onBlur?.(event);
      }}
      style={{
        display: block ? "flex" : "inline-flex",
        width: block ? "100%" : undefined,
        alignItems: "center",
        justifyContent: "center",
        gap: s.gap,
        height: s.height,
        minWidth: isLink ? undefined : s.minWidth,
        padding: isLink ? 0 : s.padding,
        boxSizing: "border-box",
        border: "none",
        borderRadius: 4,
        background: bg,
        boxShadow: shadow(border, state === "focus"),
        color,
        fontFamily: "var(--font-sans)",
        fontWeight: 500,
        fontSize: s.fontSize,
        lineHeight: s.lineHeight,
        whiteSpace: "nowrap",
        outline: "none",
        cursor: disabled ? "not-allowed" : "pointer",
        transition: "background .15s, color .15s, box-shadow .15s",
        ...style,
      }}
    >
      {leftIcon ? (
        <span style={{ display: "inline-flex", width: s.icon, height: s.icon }}>{leftIcon}</span>
      ) : null}
      <span>{children}</span>
      {rightIcon ? (
        <span style={{ display: "inline-flex", width: s.icon, height: s.icon }}>{rightIcon}</span>
      ) : null}
    </button>
  );
}

export interface IconButtonProps extends Omit<ComponentPropsWithRef<"button">, "children"> {
  icon: ReactNode;
  variant?: IconButtonVariant;
  size?: ButtonSize;
  shape?: IconButtonShape;
}

export function IconButton({
  icon,
  variant = "primary",
  size = "medium",
  shape = "circle",
  disabled = false,
  style,
  type = "button",
  onMouseEnter,
  onMouseLeave,
  onMouseDown,
  onMouseUp,
  onFocus,
  onBlur,
  ...rest
}: IconButtonProps) {
  const [state, handlers] = useInteraction(disabled);
  const s = ICON_BUTTON_SIZES[size] ?? ICON_BUTTON_SIZES.medium;
  const palette = ICON_BUTTON_VARIANTS[variant] ?? ICON_BUTTON_VARIANTS.primary;
  const [bg, border, color] = palette[paletteState(state)];
  return (
    <button
      type={type}
      disabled={disabled}
      {...rest}
      onMouseEnter={(event: MouseEvent<HTMLButtonElement>) => {
        handlers.onMouseEnter?.(event);
        onMouseEnter?.(event);
      }}
      onMouseLeave={(event: MouseEvent<HTMLButtonElement>) => {
        handlers.onMouseLeave?.(event);
        onMouseLeave?.(event);
      }}
      onMouseDown={(event: MouseEvent<HTMLButtonElement>) => {
        handlers.onMouseDown?.(event);
        onMouseDown?.(event);
      }}
      onMouseUp={(event: MouseEvent<HTMLButtonElement>) => {
        handlers.onMouseUp?.(event);
        onMouseUp?.(event);
      }}
      onFocus={(event: FocusEvent<HTMLButtonElement>) => {
        handlers.onFocus?.(event);
        onFocus?.(event);
      }}
      onBlur={(event: FocusEvent<HTMLButtonElement>) => {
        handlers.onBlur?.(event);
        onBlur?.(event);
      }}
      style={{
        width: s.box,
        height: s.box,
        padding: 0,
        display: "inline-flex",
        alignItems: "center",
        justifyContent: "center",
        border: "none",
        borderRadius: shape === "circle" ? 100 : 4,
        background: bg,
        boxShadow: shadow(border, state === "focus"),
        color,
        outline: "none",
        cursor: disabled ? "not-allowed" : "pointer",
        transition: "background .15s, color .15s, box-shadow .15s",
        ...style,
      }}
    >
      <span style={{ display: "inline-flex", width: s.icon, height: s.icon }}>{icon}</span>
    </button>
  );
}
