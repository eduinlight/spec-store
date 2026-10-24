import { screen, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { renderShell } from "~/test/shell";
import { ShortcutsDialog } from "./ShortcutsDialog";

test("a closed dialog renders nothing", () => {
  renderShell(<ShortcutsDialog open={false} onClose={vi.fn()} />);
  expect(screen.queryByRole("dialog")).not.toBeInTheDocument();
});

test("it lists every shortcut in its group with the keys", () => {
  renderShell(<ShortcutsDialog open onClose={vi.fn()} />);
  const dialog = screen.getByRole("dialog", { name: "Keyboard shortcuts" });
  for (const group of ["General", "Go to", "Project"]) {
    expect(within(dialog).getByText(group)).toBeInTheDocument();
  }
  const row = (label: string) =>
    within(dialog)
      .getAllByText(label)
      .map((element) => element.parentElement as HTMLElement)
      .find((parent) => parent.querySelector("kbd")) as HTMLElement;
  expect(row("Notifications")).toHaveTextContent("n");
  expect(row("Keyboard shortcuts")).toHaveTextContent("?");
  expect(row("Close")).toHaveTextContent("esc");
  expect(row("Search or jump to")).toHaveTextContent("K");
  const pairs: [string, string][] = [
    ["Overview", "o"],
    ["Active specs", "e"],
    ["All specs", "l"],
    ["Constitution", "c"],
    ["Activity", "a"],
    ["Members", "m"],
    ["Audit trail", "u"],
    ["Profile", "p"],
  ];
  for (const [label, key] of pairs) {
    const keys = Array.from(row(label).querySelectorAll("kbd")).map((kbd) => kbd.textContent);
    expect(keys).toEqual(["g", key]);
  }
});

test("Done and Escape close the dialog", async () => {
  const onClose = vi.fn();
  renderShell(<ShortcutsDialog open onClose={onClose} />);
  await userEvent.click(screen.getByRole("button", { name: "Done" }));
  expect(onClose).toHaveBeenCalledTimes(1);
  await userEvent.keyboard("{Escape}");
  expect(onClose).toHaveBeenCalledTimes(2);
});
