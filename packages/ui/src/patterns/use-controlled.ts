import { useCallback, useState } from "react";

export const useControlled = <T>(
  controlled: T | undefined,
  initial: T,
  onChange?: (next: T) => void,
): [T, (next: T) => void] => {
  const [inner, setInner] = useState(initial);
  const current = controlled === undefined ? inner : controlled;
  const set = useCallback(
    (next: T) => {
      if (controlled === undefined) {
        setInner(next);
      }
      onChange?.(next);
    },
    [controlled, onChange],
  );
  return [current, set];
};
