import {
  type CSSProperties,
  type ReactNode,
  type RefObject,
  useEffect,
  useId,
  useRef,
} from "react";
import { createPortal } from "react-dom";
import { useInteraction } from "./interaction";
import { CloseX } from "./StatusIcon";

const FOCUSABLE =
  'a[href], button:not([disabled]), input:not([disabled]), select:not([disabled]), textarea:not([disabled]), [tabindex]:not([tabindex="-1"])';

export function useModalFocus(
  ref: RefObject<HTMLElement | null>,
  active: boolean,
  onEscape?: () => void,
) {
  const escapeHandler = useRef(onEscape);
  escapeHandler.current = onEscape;
  useEffect(() => {
    const panel = ref.current;
    if (!active || !panel) {
      return;
    }
    const previous = document.activeElement instanceof HTMLElement ? document.activeElement : null;
    const items = () => Array.from(panel.querySelectorAll<HTMLElement>(FOCUSABLE));
    const [first] = items();
    (first ?? panel).focus();
    const onKeyDown = (event: KeyboardEvent) => {
      if (event.key === "Escape") {
        event.stopPropagation();
        escapeHandler.current?.();
        return;
      }
      if (event.key !== "Tab") {
        return;
      }
      const list = items();
      const head = list[0];
      const tail = list[list.length - 1];
      if (!head || !tail) {
        event.preventDefault();
        panel.focus();
        return;
      }
      const current = document.activeElement;
      if (!panel.contains(current)) {
        event.preventDefault();
        (event.shiftKey ? tail : head).focus();
      } else if (event.shiftKey && (current === head || current === panel)) {
        event.preventDefault();
        tail.focus();
      } else if (!event.shiftKey && current === tail) {
        event.preventDefault();
        head.focus();
      }
    };
    document.addEventListener("keydown", onKeyDown);
    return () => {
      document.removeEventListener("keydown", onKeyDown);
      previous?.focus();
    };
  }, [active, ref]);
}

export interface ModalButtonProps {
  variant?: "primary" | "outline";
  onClick?: () => void;
  children?: ReactNode;
}

export function ModalButton({ variant = "primary", onClick, children }: ModalButtonProps) {
  const [state, handlers] = useInteraction();
  const primary = variant === "primary";
  const accent = state === "hover" || state === "pressed" || state === "focus";
  let background = "#fff";
  if (primary) {
    background =
      state === "pressed"
        ? "var(--primary-700)"
        : accent
          ? "var(--primary-500)"
          : "var(--primary-600)";
  }
  let boxShadow: string | undefined;
  if (!primary) {
    boxShadow = `inset 0 0 0 1px ${accent ? "var(--primary-500)" : "var(--neutral-300)"}`;
  }
  if (state === "focus") {
    boxShadow = boxShadow ? `${boxShadow}, var(--focus-ring)` : "var(--focus-ring)";
  }
  return (
    <button
      type="button"
      onClick={onClick}
      {...handlers}
      style={{
        height: 36,
        padding: "0 16px",
        border: 0,
        borderRadius: 4,
        cursor: "pointer",
        font: "400 14px/22px var(--font-sans)",
        background,
        color: primary ? "#fff" : "var(--neutral-700)",
        boxShadow,
      }}
    >
      {children}
    </button>
  );
}

export interface ModalCloseButtonProps {
  onClick?: () => void;
  label?: string;
}

export function ModalCloseButton({ onClick, label = "Close" }: ModalCloseButtonProps) {
  return (
    <button
      type="button"
      aria-label={label}
      onClick={onClick}
      style={{
        display: "inline-flex",
        padding: 0,
        border: 0,
        background: "none",
        color: "var(--neutral-500)",
        cursor: "pointer",
      }}
    >
      <CloseX />
    </button>
  );
}

const WIDTHS = { small: 400, medium: 600, large: 880 };

export interface DialogProps {
  open?: boolean;
  title?: ReactNode;
  children?: ReactNode;
  size?: "small" | "medium" | "large" | number | string;
  okText?: ReactNode;
  cancelText?: ReactNode;
  onOk?: () => void;
  onCancel?: () => void;
  footer?: ReactNode;
  inline?: boolean;
  style?: CSSProperties;
}

export function Dialog({
  open = true,
  title = "Title",
  children = "Some contents...",
  size = "small",
  okText = "Confirm",
  cancelText = "Cancel",
  onOk,
  onCancel,
  footer,
  inline = false,
  style,
}: DialogProps) {
  const titleId = useId();
  const panelRef = useRef<HTMLDivElement>(null);
  useModalFocus(panelRef, open && !inline, onCancel);
  if (!open) {
    return null;
  }
  const width =
    typeof size === "string" && size in WIDTHS ? WIDTHS[size as keyof typeof WIDTHS] : size;
  const panel = (
    <div
      ref={panelRef}
      role="dialog"
      aria-modal={inline ? undefined : true}
      aria-labelledby={titleId}
      tabIndex={-1}
      style={{
        width,
        maxWidth: "100%",
        position: "relative",
        boxSizing: "border-box",
        padding: 24,
        borderRadius: 4,
        background: "#fff",
        boxShadow: "var(--elevation-4)",
        display: "flex",
        flexDirection: "column",
        gap: 16,
        outline: "none",
        ...style,
      }}
    >
      <div style={{ display: "flex", alignItems: "center", gap: 8 }}>
        <span
          id={titleId}
          style={{ flex: 1, font: "500 18px/26px var(--font-sans)", color: "var(--neutral-700)" }}
        >
          {title}
        </span>
        <ModalCloseButton onClick={onCancel} />
      </div>
      <div style={{ font: "400 14px/22px var(--font-sans)", color: "var(--neutral-600)" }}>
        {children}
      </div>
      {footer !== null && (
        <div style={{ display: "flex", justifyContent: "flex-end", gap: 8, marginTop: 8 }}>
          {footer ?? (
            <>
              <ModalButton variant="outline" onClick={onCancel}>
                {cancelText}
              </ModalButton>
              <ModalButton onClick={onOk}>{okText}</ModalButton>
            </>
          )}
        </div>
      )}
    </div>
  );
  if (inline) {
    return panel;
  }
  return createPortal(
    <div
      style={{
        position: "fixed",
        inset: 0,
        zIndex: 100,
        background: "var(--overlay)",
        display: "flex",
        alignItems: "center",
        justifyContent: "center",
        padding: 24,
      }}
    >
      <button
        type="button"
        tabIndex={-1}
        aria-hidden="true"
        onClick={onCancel}
        style={{
          position: "absolute",
          inset: 0,
          border: 0,
          padding: 0,
          background: "transparent",
          cursor: "default",
        }}
      />
      {panel}
    </div>,
    document.body,
  );
}
