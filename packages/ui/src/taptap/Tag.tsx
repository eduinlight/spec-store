import type {
  ComponentPropsWithRef,
  CSSProperties,
  FocusEvent,
  MouseEvent,
  MouseEventHandler,
} from "react";
import { useInteraction } from "./interaction";

export type TagType = "basic" | "dark" | "light" | "primary";
export type CheckableTagSize = "large" | "medium" | "small";
export type StatusTagSize = "small" | "medium" | "large";
export type StatusTagStatus =
  | "approved"
  | "underReview"
  | "rejected"
  | "currentVersion"
  | "cancel"
  | "installed"
  | "wishlist";

const TYPES: Record<TagType, { bg: string; border: string | null; color: string; pad: string }> = {
  basic: {
    bg: "transparent",
    border: "var(--neutral-300)",
    color: "var(--neutral-500)",
    pad: "1px 4px",
  },
  dark: { bg: "var(--auxiliary-600)", border: null, color: "#fff", pad: "1px 6px" },
  light: {
    bg: "var(--auxiliary-200)",
    border: null,
    color: "var(--auxiliary-600)",
    pad: "1px 6px",
  },
  primary: { bg: "var(--primary-100)", border: null, color: "var(--primary-600)", pad: "1px 6px" },
};

const CHECKABLE_SIZES: Record<
  CheckableTagSize,
  { h: number; pad: string; fs: number; lh: string }
> = {
  large: { h: 40, pad: "8px 12px", fs: 16, lh: "24px" },
  medium: { h: 36, pad: "7px 12px", fs: 14, lh: "22px" },
  small: { h: 24, pad: "3px 8px", fs: 12, lh: "18px" },
};

const STATUS: Record<StatusTagStatus, readonly [string, string, string]> = {
  approved: ["var(--success-600)", "var(--success-100)", "Approved"],
  underReview: ["var(--warning-600)", "var(--warning-100)", "Under Review"],
  rejected: ["var(--danger-600)", "var(--danger-100)", "Rejected"],
  currentVersion: ["var(--info-600)", "var(--info-100)", "Current Version"],
  cancel: ["var(--neutral-400)", "var(--neutral-100)", "Cancel"],
  installed: ["var(--info-600)", "var(--info-100)", "Installed"],
  wishlist: ["var(--warning-600)", "var(--warning-100)", "Wishlist"],
};

const STATUS_SIZES: Record<StatusTagSize, readonly [number, number, string, number]> = {
  small: [24, 12, "18px", 16],
  medium: [28, 14, "22px", 18],
  large: [32, 16, "24px", 20],
};

const FOCUS_RING = "var(--focus-ring)";

export interface TagProps extends ComponentPropsWithRef<"span"> {
  type?: TagType;
  color?: string;
  closable?: boolean;
  onClose?: MouseEventHandler<HTMLButtonElement>;
  closeLabel?: string;
}

interface TagCloseButtonProps {
  label: string;
  onClose?: MouseEventHandler<HTMLButtonElement>;
}

function TagCloseButton({ label, onClose }: TagCloseButtonProps) {
  const [state, handlers] = useInteraction();
  return (
    <button
      type="button"
      aria-label={label}
      onClick={onClose}
      {...handlers}
      style={{
        display: "inline-flex",
        alignItems: "center",
        justifyContent: "center",
        padding: 0,
        margin: 0,
        border: "none",
        borderRadius: 2,
        background: "transparent",
        outline: "none",
        boxShadow: state === "focus" ? FOCUS_RING : "none",
        cursor: "pointer",
        color:
          state === "hover" || state === "pressed" ? "var(--neutral-600)" : "var(--neutral-500)",
      }}
    >
      <svg aria-hidden="true" focusable="false" width="10" height="10" viewBox="0 0 12.5 12.5">
        <path
          fill="currentColor"
          d="M5.421 6.246 0 11.667l.825.825 5.421-5.421 5.421 5.421.825-.825-5.421-5.421L12.492.825 11.667 0 6.246 5.421.825 0 0 .825z"
        />
      </svg>
    </button>
  );
}

export function Tag({
  type = "basic",
  color,
  closable = false,
  onClose,
  closeLabel = "Remove",
  children = "Tag",
  style,
  ...rest
}: TagProps) {
  const t = TYPES[type] ?? TYPES.basic;
  return (
    <span
      {...rest}
      style={{
        display: "inline-flex",
        alignItems: "center",
        gap: 4,
        height: closable ? 24 : 20,
        boxSizing: "border-box",
        padding: closable ? "3px 8px" : t.pad,
        borderRadius: 2,
        background: color && type === "dark" ? color : t.bg,
        boxShadow: t.border ? `inset 0 0 0 1px ${t.border}` : "none",
        color: t.color,
        font: "400 12px/18px var(--font-sans)",
        whiteSpace: "nowrap",
        ...style,
      }}
    >
      {children}
      {closable ? <TagCloseButton label={closeLabel} onClose={onClose} /> : null}
    </span>
  );
}

export interface CheckableTagProps
  extends Omit<ComponentPropsWithRef<"button">, "onChange" | "type" | "checked"> {
  checked?: boolean;
  size?: CheckableTagSize;
  onChange?: (checked: boolean) => void;
}

export function CheckableTag({
  checked = false,
  disabled = false,
  size = "medium",
  onChange,
  onClick,
  children = "Tag",
  style,
  onMouseEnter,
  onMouseLeave,
  onMouseDown,
  onMouseUp,
  onFocus,
  onBlur,
  ...rest
}: CheckableTagProps) {
  const [state, handlers] = useInteraction(disabled);
  const s = CHECKABLE_SIZES[size] ?? CHECKABLE_SIZES.medium;
  const active = state === "hover" || state === "pressed";
  const border = disabled
    ? "var(--neutral-300)"
    : checked
      ? "var(--primary-600)"
      : active
        ? "var(--primary-500)"
        : "var(--neutral-300)";
  const color = disabled
    ? "var(--neutral-400)"
    : checked
      ? "var(--primary-600)"
      : active
        ? "var(--primary-500)"
        : "var(--neutral-600)";
  const shadow: CSSProperties["boxShadow"] =
    state === "focus" ? `inset 0 0 0 1px ${border}, ${FOCUS_RING}` : `inset 0 0 0 1px ${border}`;
  return (
    <button
      type="button"
      aria-pressed={checked}
      disabled={disabled}
      {...rest}
      onClick={(event) => {
        onClick?.(event);
        onChange?.(!checked);
      }}
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
        position: "relative",
        overflow: "hidden",
        display: "inline-flex",
        alignItems: "center",
        margin: 0,
        height: s.h,
        boxSizing: "border-box",
        padding: s.pad,
        border: "none",
        outline: "none",
        borderRadius: 4,
        background: disabled ? "var(--neutral-100)" : "#fff",
        boxShadow: shadow,
        color,
        font: `400 ${s.fs}px/${s.lh} var(--font-sans)`,
        cursor: disabled ? "not-allowed" : "pointer",
        transition: "box-shadow .15s, color .15s",
        ...style,
      }}
    >
      {children}
      {checked ? (
        <span
          aria-hidden="true"
          style={{
            position: "absolute",
            right: 0,
            bottom: 0,
            width: 0,
            height: 0,
            borderStyle: "solid",
            borderWidth: `0 0 ${s.h / 2}px ${s.h / 2}px`,
            borderColor: `transparent transparent ${disabled ? "var(--neutral-300)" : "var(--primary-600)"} transparent`,
          }}
        />
      ) : null}
    </button>
  );
}

export interface StatusTagProps extends ComponentPropsWithRef<"span"> {
  status?: StatusTagStatus;
  size?: StatusTagSize;
}

export function StatusTag({
  status = "approved",
  size = "small",
  children,
  style,
  ...rest
}: StatusTagProps) {
  const [c, bg, label] = STATUS[status] ?? STATUS.approved;
  const big = STATUS_SIZES[size] ?? STATUS_SIZES.small;
  return (
    <span
      {...rest}
      style={{
        display: "inline-flex",
        alignItems: "center",
        gap: 4,
        height: big[0],
        boxSizing: "border-box",
        padding: "3px 12px 3px 4px",
        borderRadius: 100,
        background: bg,
        color: c,
        font: `400 ${big[1]}px/${big[2]} var(--font-sans)`,
        whiteSpace: "nowrap",
        ...style,
      }}
    >
      <span
        aria-hidden="true"
        style={{
          width: big[3],
          height: big[3],
          borderRadius: "50%",
          background: c,
          display: "inline-flex",
          alignItems: "center",
          justifyContent: "center",
        }}
      >
        <span
          style={{
            width: big[3] / 2,
            height: big[3] / 2,
            borderRadius: "50%",
            background: "rgba(255,255,255,0.55)",
          }}
        />
      </span>
      {children ?? label}
    </span>
  );
}
