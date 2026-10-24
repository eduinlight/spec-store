import {
  type CSSProperties,
  cloneElement,
  isValidElement,
  type KeyboardEvent,
  type ReactElement,
  type ReactNode,
  useId,
  useState,
} from "react";

export type TooltipPlacement = "top" | "bottom" | "left" | "right";

const POS: Record<TooltipPlacement, CSSProperties> = {
  top: {
    bottom: "100%",
    left: "50%",
    transform: "translateX(-50%)",
    marginBottom: 8,
  },
  bottom: {
    top: "100%",
    left: "50%",
    transform: "translateX(-50%)",
    marginTop: 8,
  },
  left: {
    right: "100%",
    top: "50%",
    transform: "translateY(-50%)",
    marginRight: 8,
  },
  right: {
    left: "100%",
    top: "50%",
    transform: "translateY(-50%)",
    marginLeft: 8,
  },
};

const ARROW: Record<TooltipPlacement, CSSProperties> = {
  top: {
    top: "100%",
    left: "50%",
    marginLeft: -5,
    borderWidth: "5px 5px 0",
    borderColor: "var(--c) transparent transparent",
  },
  bottom: {
    bottom: "100%",
    left: "50%",
    marginLeft: -5,
    borderWidth: "0 5px 5px",
    borderColor: "transparent transparent var(--c)",
  },
  left: {
    left: "100%",
    top: "50%",
    marginTop: -5,
    borderWidth: "5px 0 5px 5px",
    borderColor: "transparent transparent transparent var(--c)",
  },
  right: {
    right: "100%",
    top: "50%",
    marginTop: -5,
    borderWidth: "5px 5px 5px 0",
    borderColor: "transparent var(--c) transparent transparent",
  },
};

export interface FloatingProps {
  placement: TooltipPlacement;
  color: string;
  children?: ReactNode;
  panelStyle?: CSSProperties;
  id?: string;
  role?: string;
}

export const Floating = ({ placement, color, children, panelStyle, id, role }: FloatingProps) => (
  <span
    style={{
      position: "absolute",
      zIndex: 30,
      ...POS[placement],
      ["--c" as string]: color,
    }}
  >
    <span id={id} role={role} style={{ position: "relative", display: "block", ...panelStyle }}>
      {children}
      <span
        aria-hidden="true"
        style={{
          position: "absolute",
          width: 0,
          height: 0,
          borderStyle: "solid",
          ...ARROW[placement],
        }}
      />
    </span>
  </span>
);

export interface TooltipProps {
  title?: ReactNode;
  placement?: TooltipPlacement;
  open?: boolean;
  children?: ReactNode;
  style?: CSSProperties;
}

export const Tooltip = ({ title = "", placement = "top", open, children, style }: TooltipProps) => {
  const [hover, setHover] = useState(false);
  const [focus, setFocus] = useState(false);
  const bubbleId = useId();
  const show = Boolean(open ?? (hover || focus)) && Boolean(title);

  const dismiss = (event: KeyboardEvent) => {
    if (event.key === "Escape") {
      setHover(false);
      setFocus(false);
    }
  };

  const describedChildren =
    show && isValidElement(children)
      ? cloneElement(children as ReactElement<{ "aria-describedby"?: string }>, {
          "aria-describedby": bubbleId,
        })
      : children;

  return (
    <span
      role="none"
      onMouseEnter={() => setHover(true)}
      onMouseLeave={() => setHover(false)}
      onFocus={() => setFocus(true)}
      onBlur={() => setFocus(false)}
      onKeyDown={dismiss}
      aria-describedby={show && !isValidElement(children) ? bubbleId : undefined}
      style={{ position: "relative", display: "inline-flex", ...style }}
    >
      {describedChildren}
      {show && (
        <Floating
          placement={placement}
          color="#000"
          id={bubbleId}
          role="tooltip"
          panelStyle={{
            background: "#000",
            color: "#fff",
            padding: 8,
            borderRadius: 4,
            font: "400 12px/18px var(--font-sans)",
            whiteSpace: "normal",
            width: "max-content",
            maxWidth: 260,
            textAlign: "left",
            textWrap: "pretty",
          }}
        >
          {title}
        </Floating>
      )}
    </span>
  );
};
