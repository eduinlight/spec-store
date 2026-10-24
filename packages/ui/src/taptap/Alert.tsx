import type { CSSProperties, ReactNode } from "react";
import { CloseX, StatusIcon, type StatusKind } from "./StatusIcon";

const TONE: Record<StatusKind, [string, string]> = {
  info: ["var(--info-100)", "var(--info-300)"],
  success: ["var(--success-100)", "var(--success-300)"],
  warning: ["var(--warning-100)", "var(--warning-300)"],
  error: ["var(--danger-100)", "var(--danger-300)"],
};

export interface AlertProps {
  status?: StatusKind;
  title?: ReactNode;
  children?: ReactNode;
  size?: "default" | "small";
  action?: ReactNode;
  closable?: boolean;
  onClose?: () => void;
  showIcon?: boolean;
  closeLabel?: string;
  style?: CSSProperties;
}

export function Alert({
  status = "info",
  title,
  children = "Info Text",
  size = "default",
  action,
  closable = true,
  onClose,
  showIcon = true,
  closeLabel = "Close",
  style,
}: AlertProps) {
  const [background, border] = TONE[status] ?? TONE.info;
  const small = size === "small";
  const multi = Boolean(title);
  const urgent = status === "error" || status === "warning";
  return (
    <div
      role={urgent ? "alert" : "status"}
      style={{
        display: "flex",
        alignItems: multi ? "flex-start" : "center",
        gap: 12,
        minHeight: small ? 32 : 40,
        boxSizing: "border-box",
        padding: multi ? "12px 16px" : small ? "5px 12px" : "9px 16px",
        borderRadius: 4,
        background,
        boxShadow: `inset 0 0 0 1px ${border}`,
        ...style,
      }}
    >
      {showIcon && (
        <span style={{ display: "inline-flex", marginTop: multi ? 2 : 0 }}>
          <StatusIcon status={status} size={small ? 14 : 18} />
        </span>
      )}
      <div style={{ flex: 1, minWidth: 0 }}>
        {title && (
          <div
            style={{
              font: "500 16px/24px var(--font-sans)",
              color: "var(--neutral-700)",
              marginBottom: 4,
            }}
          >
            {title}
          </div>
        )}
        <div
          style={{
            font: `400 ${small ? 12 : 14}px/${small ? "18px" : "22px"} var(--font-sans)`,
            color: multi ? "var(--neutral-600)" : "var(--neutral-700)",
          }}
        >
          {children}
        </div>
      </div>
      {(action || closable) && (
        <div style={{ display: "flex", alignItems: "center", gap: 16 }}>
          {action}
          {closable && (
            <button
              type="button"
              aria-label={closeLabel}
              onClick={onClose}
              style={{
                display: "inline-flex",
                padding: 0,
                border: 0,
                background: "none",
                color: "var(--neutral-500)",
                cursor: "pointer",
              }}
            >
              <CloseX size={small ? 12 : 14} />
            </button>
          )}
        </div>
      )}
    </div>
  );
}
