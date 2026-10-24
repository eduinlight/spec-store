import { type FocusEvent, type MouseEvent, useState } from "react";

export type InteractionState = "default" | "hover" | "pressed" | "focus" | "disabled";

export interface InteractionHandlers {
  onMouseEnter?: (event: MouseEvent) => void;
  onMouseLeave?: (event: MouseEvent) => void;
  onMouseDown?: (event: MouseEvent) => void;
  onMouseUp?: (event: MouseEvent) => void;
  onFocus?: (event: FocusEvent) => void;
  onBlur?: (event: FocusEvent) => void;
}

export function useInteraction(disabled = false): [InteractionState, InteractionHandlers] {
  const [hover, setHover] = useState(false);
  const [press, setPress] = useState(false);
  const [focus, setFocus] = useState(false);
  if (disabled) {
    return ["disabled", {}];
  }
  const handlers: InteractionHandlers = {
    onMouseEnter: () => setHover(true),
    onMouseLeave: () => {
      setHover(false);
      setPress(false);
    },
    onMouseDown: () => setPress(true),
    onMouseUp: () => setPress(false),
    onFocus: () => setFocus(true),
    onBlur: () => setFocus(false),
  };
  const state: InteractionState = press ? "pressed" : hover ? "hover" : focus ? "focus" : "default";
  return [state, handlers];
}
