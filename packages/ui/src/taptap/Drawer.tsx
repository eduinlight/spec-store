import { type CSSProperties, type ReactNode, useId, useRef } from "react";
import { createPortal } from "react-dom";
import { ModalButton, ModalCloseButton, useModalFocus } from "./Dialog";

export interface DrawerProps {
  open?: boolean;
  title?: ReactNode;
  children?: ReactNode;
  width?: number | string;
  placement?: "left" | "right";
  footer?: ReactNode;
  onClose?: () => void;
  okText?: ReactNode;
  cancelText?: ReactNode;
  onOk?: () => void;
  inline?: boolean;
  style?: CSSProperties;
}

export function Drawer({
  open = true,
  title = "Title",
  children,
  width = 480,
  placement = "right",
  footer,
  onClose,
  okText = "Confirm",
  cancelText = "Cancel",
  onOk,
  inline = false,
  style,
}: DrawerProps) {
  const titleId = useId();
  const panelRef = useRef<HTMLDivElement>(null);
  useModalFocus(panelRef, open && !inline, onClose);
  if (!open) {
    return null;
  }
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
        height: "100%",
        position: "relative",
        boxSizing: "border-box",
        background: "#fff",
        boxShadow: "var(--elevation-4)",
        display: "flex",
        flexDirection: "column",
        outline: "none",
        ...style,
      }}
    >
      <div
        style={{
          display: "flex",
          alignItems: "center",
          gap: 8,
          padding: "16px 24px",
          borderBottom: "1px solid var(--neutral-200)",
        }}
      >
        <span
          id={titleId}
          style={{ flex: 1, font: "500 18px/26px var(--font-sans)", color: "var(--neutral-700)" }}
        >
          {title}
        </span>
        <ModalCloseButton onClick={onClose} />
      </div>
      <div
        style={{
          flex: 1,
          overflow: "auto",
          padding: 24,
          font: "400 14px/22px var(--font-sans)",
          color: "var(--neutral-600)",
        }}
      >
        {children}
      </div>
      {footer !== null && (
        <div
          style={{
            display: "flex",
            justifyContent: "flex-end",
            gap: 8,
            padding: "12px 24px",
            borderTop: "1px solid var(--neutral-200)",
          }}
        >
          {footer ?? (
            <>
              <ModalButton variant="outline" onClick={onClose}>
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
        justifyContent: placement === "left" ? "flex-start" : "flex-end",
      }}
    >
      <button
        type="button"
        tabIndex={-1}
        aria-hidden="true"
        onClick={onClose}
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
