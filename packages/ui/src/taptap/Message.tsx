import { type CSSProperties, type ReactNode, useSyncExternalStore } from "react";
import { StatusIcon, type StatusKind } from "./StatusIcon";

export interface MessageProps {
  status?: StatusKind;
  children?: ReactNode;
  style?: CSSProperties;
}

export function Message({ status = "info", children = "This is a message", style }: MessageProps) {
  return (
    <div
      role="status"
      style={{
        display: "inline-flex",
        alignItems: "center",
        gap: 8,
        padding: "8px 16px",
        borderRadius: 4,
        background: "#fff",
        boxShadow: "var(--elevation-3)",
        font: "400 14px/22px var(--font-sans)",
        color: "var(--neutral-700)",
        animation: "tt-fade-in .2s",
        ...style,
      }}
    >
      <StatusIcon status={status} size={16} />
      {children}
    </div>
  );
}

export interface MessageOptions {
  duration?: number;
}

interface Toast {
  id: number;
  status: StatusKind;
  text: ReactNode;
}

const DEFAULT_DURATION = 3000;

let toasts: Toast[] = [];
let nextId = 1;
const listeners = new Set<() => void>();

const emit = () => {
  for (const listener of listeners) {
    listener();
  }
};

const subscribe = (listener: () => void) => {
  listeners.add(listener);
  return () => {
    listeners.delete(listener);
  };
};

const snapshot = () => toasts;

const dismiss = (id: number) => {
  toasts = toasts.filter((toast) => toast.id !== id);
  emit();
};

const show = (status: StatusKind, text: ReactNode, options: MessageOptions = {}) => {
  const id = nextId++;
  toasts = [...toasts, { id, status, text }];
  emit();
  setTimeout(() => dismiss(id), options.duration ?? DEFAULT_DURATION);
  return id;
};

export const message = {
  success: (text: ReactNode, options?: MessageOptions) => show("success", text, options),
  error: (text: ReactNode, options?: MessageOptions) => show("error", text, options),
  info: (text: ReactNode, options?: MessageOptions) => show("info", text, options),
  warning: (text: ReactNode, options?: MessageOptions) => show("warning", text, options),
  dismiss,
};

export function MessageHost() {
  const current = useSyncExternalStore(subscribe, snapshot, snapshot);
  return (
    <div
      role="status"
      aria-live="polite"
      aria-label="Notifications"
      style={{
        position: "fixed",
        top: 16,
        left: 0,
        right: 0,
        zIndex: 200,
        display: "flex",
        flexDirection: "column",
        alignItems: "center",
        gap: 8,
        pointerEvents: "none",
      }}
    >
      {current.map((toast) => (
        <Message key={toast.id} status={toast.status} style={{ pointerEvents: "auto" }}>
          {toast.text}
        </Message>
      ))}
    </div>
  );
}
