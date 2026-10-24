import { renderHook } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { type ShortcutHandlers, useShortcuts } from "./useShortcuts";

function handlers(): ShortcutHandlers {
  return { onPalette: vi.fn() };
}

test("Ctrl+K and Cmd+K open the palette even while typing", async () => {
  const subject = handlers();
  renderHook(() => useShortcuts(subject));
  const input = document.createElement("input");
  document.body.append(input);
  input.focus();
  await userEvent.keyboard("{Control>}k{/Control}");
  await userEvent.keyboard("{Meta>}k{/Meta}");
  expect(subject.onPalette).toHaveBeenCalledTimes(2);
  input.remove();
});

test("the palette shortcut is released when the component unmounts", async () => {
  const subject = handlers();
  const { unmount } = renderHook(() => useShortcuts(subject));
  unmount();
  await userEvent.keyboard("{Control>}k{/Control}");
  expect(subject.onPalette).not.toHaveBeenCalled();
});
