import { useEffect, useRef } from "react";

export interface ShortcutHandlers {
  onPalette: () => void;
  onShortcuts: () => void;
  onNotifications: () => void;
  onEscape: () => void;
  onGo: (chord: string) => void;
}

const CHORD_TIMEOUT_MS = 1500;
const CHORD_KEYS = new Set(["o", "e", "l", "c", "a", "m", "u", "p"]);
const TEXT_TAGS = new Set(["INPUT", "TEXTAREA", "SELECT"]);

const isTextTarget = (target: EventTarget | null): boolean =>
  target instanceof HTMLElement &&
  (TEXT_TAGS.has(target.tagName) ||
    target.isContentEditable === true ||
    target.closest('[contenteditable=""], [contenteditable="true"]') !== null);

const isModalOpen = (): boolean => document.querySelector('[aria-modal="true"]') !== null;

export function useShortcuts(handlers: ShortcutHandlers) {
  const latest = useRef(handlers);
  latest.current = handlers;
  useEffect(() => {
    let chordStartedAt = 0;
    const onKeyDown = (event: KeyboardEvent) => {
      const modified = event.metaKey || event.ctrlKey;
      if (modified && event.key.toLowerCase() === "k") {
        event.preventDefault();
        latest.current.onPalette();
        return;
      }
      if (modified || event.altKey || isTextTarget(event.target)) {
        return;
      }
      if (event.key === "Escape") {
        chordStartedAt = 0;
        latest.current.onEscape();
        return;
      }
      if (isModalOpen()) {
        chordStartedAt = 0;
        return;
      }
      if (chordStartedAt > 0) {
        const fresh = Date.now() - chordStartedAt < CHORD_TIMEOUT_MS;
        chordStartedAt = 0;
        if (fresh && CHORD_KEYS.has(event.key)) {
          event.preventDefault();
          latest.current.onGo(event.key);
        }
        return;
      }
      if (event.key === "g") {
        chordStartedAt = Date.now();
      } else if (event.key === "?") {
        latest.current.onShortcuts();
      } else if (event.key === "n") {
        latest.current.onNotifications();
      }
    };
    document.addEventListener("keydown", onKeyDown);
    return () => document.removeEventListener("keydown", onKeyDown);
  }, []);
}
