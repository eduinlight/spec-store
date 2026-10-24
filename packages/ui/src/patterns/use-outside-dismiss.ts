import { type RefObject, useEffect } from "react";

export const useOutsideDismiss = (
  containerRef: RefObject<HTMLElement | null>,
  active: boolean,
  onDismiss: () => void,
) => {
  useEffect(() => {
    if (!active) {
      return;
    }
    const handle = (event: MouseEvent) => {
      if (containerRef.current && !containerRef.current.contains(event.target as Node)) {
        onDismiss();
      }
    };
    document.addEventListener("mousedown", handle);
    return () => document.removeEventListener("mousedown", handle);
  }, [active, containerRef, onDismiss]);
};
