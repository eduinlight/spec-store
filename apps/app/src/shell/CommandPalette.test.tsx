import { screen, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { renderShell } from "~/test/shell";
import { CommandPalette, type CommandPaletteProps } from "./CommandPalette";

const specs = [
  { number: 4, slug: "agent-origin", branch: "004-agent-origin", status: "approved" },
  { number: 5, slug: "saas-foundation", branch: "005-saas-foundation", status: "draft" },
];

function props(overrides: Partial<CommandPaletteProps> = {}): CommandPaletteProps {
  return {
    open: true,
    onClose: vi.fn(),
    org: "acme",
    project: "demo",
    specs,
    active: {
      number: 5,
      slug: "saas-foundation",
      branch: "005-saas-foundation",
      nextTask: { id: "T13", description: "Migrate refunds" },
    },
    onOpenNotifications: vi.fn(),
    onOpenShortcuts: vi.fn(),
    ...overrides,
  };
}

const labels = () => screen.getAllByRole("option").map((option) => option.textContent);

test("a closed palette renders nothing", () => {
  renderShell(<CommandPalette {...props({ open: false })} />);
  expect(screen.queryByRole("dialog")).not.toBeInTheDocument();
});

test("it opens with the search field focused and the three groups", () => {
  renderShell(<CommandPalette {...props()} />, "/p/acme/demo");
  const dialog = screen.getByRole("dialog", { name: "Command palette" });
  const input = within(dialog).getByRole("combobox");
  expect(input).toHaveFocus();
  expect(within(dialog).getByRole("group", { name: "Suggested" })).toBeInTheDocument();
  expect(within(dialog).getByRole("group", { name: "Specs" })).toBeInTheDocument();
  expect(within(dialog).getByRole("group", { name: "Navigation" })).toBeInTheDocument();
  const suggested = within(within(dialog).getByRole("group", { name: "Suggested" }));
  expect(suggested.getAllByRole("option")[0]).toHaveTextContent("T13");
  expect(suggested.getAllByRole("option")[0]).toHaveTextContent("Next task · Migrate refunds");
  const specOptions = within(within(dialog).getByRole("group", { name: "Specs" })).getAllByRole(
    "option",
  );
  expect(specOptions.map((option) => option.textContent)).toEqual([
    expect.stringContaining("005"),
    expect.stringContaining("004"),
  ]);
  expect(specOptions[0]).toHaveTextContent("saas-foundation · Draft");
  expect(within(dialog).getByRole("option", { name: /Active specs/ })).toHaveTextContent("g");
});

test("the first result is active and arrows move the selection", async () => {
  renderShell(<CommandPalette {...props()} />, "/p/acme/demo");
  const input = screen.getByRole("combobox");
  const options = screen.getAllByRole("option");
  expect(options[0]).toHaveAttribute("aria-selected", "true");
  expect(input).toHaveAttribute("aria-activedescendant", options[0]?.id);
  await userEvent.keyboard("{ArrowDown}{ArrowDown}");
  expect(screen.getAllByRole("option")[2]).toHaveAttribute("aria-selected", "true");
  expect(input).toHaveAttribute("aria-activedescendant", screen.getAllByRole("option")[2]?.id);
  await userEvent.keyboard("{ArrowUp}");
  expect(screen.getAllByRole("option")[1]).toHaveAttribute("aria-selected", "true");
  await userEvent.keyboard("{ArrowUp}{ArrowUp}{ArrowUp}");
  expect(screen.getAllByRole("option")[0]).toHaveAttribute("aria-selected", "true");
});

test("typing filters specs and navigation and hides the suggestions", async () => {
  renderShell(<CommandPalette {...props()} />, "/p/acme/demo");
  await userEvent.type(screen.getByRole("combobox"), "found");
  expect(screen.queryByRole("group", { name: "Suggested" })).not.toBeInTheDocument();
  expect(labels()).toEqual([expect.stringContaining("005")]);
  await userEvent.clear(screen.getByRole("combobox"));
  await userEvent.type(screen.getByRole("combobox"), "audit");
  expect(labels().every((label) => /audit/i.test(label ?? ""))).toBe(true);
  expect(labels().length).toBeGreaterThan(0);
});

test("a search without matches says so", async () => {
  renderShell(<CommandPalette {...props()} />, "/p/acme/demo");
  await userEvent.type(screen.getByRole("combobox"), "zzz");
  expect(screen.getByText("No results for “zzz”.")).toBeInTheDocument();
  expect(screen.queryAllByRole("option")).toEqual([]);
});

test("Enter opens the selected result and closes the palette", async () => {
  const handlers = props();
  renderShell(<CommandPalette {...handlers} />, "/p/acme/demo");
  await userEvent.keyboard("{Enter}");
  expect(screen.getByTestId("location")).toHaveTextContent("/p/acme/demo/specs/5/tasks");
  expect(handlers.onClose).toHaveBeenCalled();
});

test("choosing a spec with the mouse opens it", async () => {
  renderShell(<CommandPalette {...props()} />, "/p/acme/demo");
  await userEvent.click(screen.getByRole("option", { name: /004/ }));
  expect(screen.getByTestId("location")).toHaveTextContent("/p/acme/demo/specs/4");
});

test("navigation results open screens of the project, the account and the organization", async () => {
  renderShell(<CommandPalette {...props()} />, "/p/acme/demo");
  const input = screen.getByRole("combobox");
  await userEvent.type(input, "billing");
  await userEvent.keyboard("{Enter}");
  expect(screen.getByTestId("location")).toHaveTextContent("/o/acme/billing");
});

test("action results call their handlers", async () => {
  const handlers = props();
  renderShell(<CommandPalette {...handlers} />, "/p/acme/demo");
  await userEvent.click(screen.getByRole("option", { name: /Open notifications/ }));
  expect(handlers.onOpenNotifications).toHaveBeenCalledTimes(1);
  expect(handlers.onClose).toHaveBeenCalled();
});

test("Escape and a click outside close the palette", async () => {
  const handlers = props();
  renderShell(<CommandPalette {...handlers} />, "/p/acme/demo");
  await userEvent.keyboard("{Escape}");
  expect(handlers.onClose).toHaveBeenCalledTimes(1);
  await userEvent.click(screen.getByTestId("palette-backdrop"));
  expect(handlers.onClose).toHaveBeenCalledTimes(2);
});

test("without a project there are no specs and only account and organization screens", () => {
  renderShell(
    <CommandPalette {...props({ project: undefined, specs: [], active: undefined })} />,
    "/o/acme",
  );
  expect(screen.queryByRole("group", { name: "Specs" })).not.toBeInTheDocument();
  expect(screen.queryByRole("option", { name: /Active specs/ })).not.toBeInTheDocument();
  expect(screen.getByRole("option", { name: /Profile/ })).toBeInTheDocument();
  expect(screen.getByRole("option", { name: /Security/ })).toBeInTheDocument();
});
