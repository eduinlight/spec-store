import { renderHook } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { afterEach } from "vitest";
import { type ShortcutHandlers, useShortcuts } from "./useShortcuts";

function handlers(): ShortcutHandlers {
  return {
    onPalette: vi.fn(),
    onShortcuts: vi.fn(),
    onNotifications: vi.fn(),
    onEscape: vi.fn(),
    onGo: vi.fn(),
  };
}

const added: HTMLElement[] = [];

function focusable(element: HTMLElement) {
  document.body.append(element);
  added.push(element);
  element.focus();
  return element;
}

afterEach(() => {
  for (const element of added.splice(0)) {
    element.remove();
  }
});

test("Ctrl+K and Cmd+K open the palette even while typing", async () => {
  const subject = handlers();
  renderHook(() => useShortcuts(subject));
  focusable(document.createElement("input"));
  await userEvent.keyboard("{Control>}k{/Control}");
  await userEvent.keyboard("{Meta>}k{/Meta}");
  expect(subject.onPalette).toHaveBeenCalledTimes(2);
});

test("the palette shortcut is released when the component unmounts", async () => {
  const subject = handlers();
  const { unmount } = renderHook(() => useShortcuts(subject));
  unmount();
  await userEvent.keyboard("{Control>}k{/Control}");
  expect(subject.onPalette).not.toHaveBeenCalled();
});

test("? lists the shortcuts, n opens the notifications and Escape closes", async () => {
  const subject = handlers();
  renderHook(() => useShortcuts(subject));
  await userEvent.keyboard("?");
  await userEvent.keyboard("n");
  await userEvent.keyboard("{Escape}");
  expect(subject.onShortcuts).toHaveBeenCalledTimes(1);
  expect(subject.onNotifications).toHaveBeenCalledTimes(1);
  expect(subject.onEscape).toHaveBeenCalledTimes(1);
});

test("g followed by a key reports the chord", async () => {
  const subject = handlers();
  renderHook(() => useShortcuts(subject));
  await userEvent.keyboard("go");
  await userEvent.keyboard("ga");
  expect(subject.onGo).toHaveBeenNthCalledWith(1, "o");
  expect(subject.onGo).toHaveBeenNthCalledWith(2, "a");
});

test("a lone g, an unknown second key or a doubled g do not navigate", async () => {
  const subject = handlers();
  renderHook(() => useShortcuts(subject));
  await userEvent.keyboard("g");
  await userEvent.keyboard("n");
  await userEvent.keyboard("gg");
  await userEvent.keyboard("o");
  expect(subject.onGo).not.toHaveBeenCalled();
});

test("a chord expires after a moment", async () => {
  vi.useFakeTimers({ shouldAdvanceTime: true });
  const subject = handlers();
  renderHook(() => useShortcuts(subject));
  await userEvent.keyboard("g");
  vi.advanceTimersByTime(2000);
  await userEvent.keyboard("o");
  vi.useRealTimers();
  expect(subject.onGo).not.toHaveBeenCalled();
});

test("shortcuts are ignored in inputs, textareas, selects and editable content", async () => {
  const subject = handlers();
  renderHook(() => useShortcuts(subject));
  const editable = document.createElement("div");
  editable.setAttribute("contenteditable", "true");
  editable.tabIndex = 0;
  const targets = [
    document.createElement("input"),
    document.createElement("textarea"),
    document.createElement("select"),
    editable,
  ];
  for (const target of targets) {
    focusable(target);
    await userEvent.keyboard("?ngo{Escape}");
  }
  expect(subject.onShortcuts).not.toHaveBeenCalled();
  expect(subject.onNotifications).not.toHaveBeenCalled();
  expect(subject.onGo).not.toHaveBeenCalled();
  expect(subject.onEscape).not.toHaveBeenCalled();
});

test("letters pressed with Ctrl, Cmd or Alt are not shortcuts", async () => {
  const subject = handlers();
  renderHook(() => useShortcuts(subject));
  await userEvent.keyboard("{Control>}n{/Control}");
  await userEvent.keyboard("{Alt>}n{/Alt}");
  expect(subject.onNotifications).not.toHaveBeenCalled();
});

test("only Escape works while a modal is open", async () => {
  const subject = handlers();
  renderHook(() => useShortcuts(subject));
  const modal = focusable(document.createElement("div"));
  modal.setAttribute("aria-modal", "true");
  await userEvent.keyboard("n?go{Escape}");
  expect(subject.onNotifications).not.toHaveBeenCalled();
  expect(subject.onShortcuts).not.toHaveBeenCalled();
  expect(subject.onGo).not.toHaveBeenCalled();
  expect(subject.onEscape).toHaveBeenCalledTimes(1);
});
