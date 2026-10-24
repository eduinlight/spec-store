import type { MouseEvent } from "react";
import type { NavigateFunction } from "react-router";

const isPlainClick = (event: MouseEvent<HTMLElement>): boolean =>
  event.button === 0 && !event.metaKey && !event.ctrlKey && !event.shiftKey && !event.altKey;

export const followLink =
  (navigate: NavigateFunction, path: string, after?: () => void) =>
  (event: MouseEvent<HTMLElement>) => {
    if (!isPlainClick(event)) {
      return;
    }
    event.preventDefault();
    navigate(path);
    after?.();
  };
