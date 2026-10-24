import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { useState } from "react";
import { Tabs, type TabsItem } from "./Tabs";

const ITEMS: TabsItem[] = [
  { label: "Spec", value: "spec" },
  { label: "Plan", value: "plan" },
  { label: "Tasks", value: "tasks", disabled: true },
  { label: "Research", value: "research" },
];

const Controlled = ({ type }: { type?: "line" | "card" }) => {
  const [value, setValue] = useState("spec");
  return (
    <Tabs items={ITEMS} value={value} onChange={(next) => setValue(String(next))} type={type} />
  );
};

describe("Tabs", () => {
  it("exposes a tablist of tabs with the selected one marked", () => {
    render(<Controlled />);
    expect(screen.getByRole("tablist")).toBeInTheDocument();
    const tabs = screen.getAllByRole("tab");
    expect(tabs).toHaveLength(4);
    expect(tabs[0]).toHaveAttribute("aria-selected", "true");
    expect(tabs[1]).toHaveAttribute("aria-selected", "false");
    expect(tabs[2]).toHaveAttribute("aria-disabled", "true");
  });

  it("uses a roving tabindex", () => {
    render(<Controlled />);
    const tabs = screen.getAllByRole("tab");
    expect(tabs.map((tab) => tab.getAttribute("tabindex"))).toEqual(["0", "-1", "-1", "-1"]);
  });

  it("moves with the arrow keys, Home and End and skips disabled tabs", async () => {
    const user = userEvent.setup();
    render(<Controlled />);
    const tabs = screen.getAllByRole("tab");
    tabs[0]?.focus();
    await user.keyboard("{ArrowRight}");
    expect(tabs[1]).toHaveFocus();
    expect(tabs[1]).toHaveAttribute("aria-selected", "true");
    await user.keyboard("{ArrowRight}");
    expect(tabs[3]).toHaveFocus();
    expect(tabs[3]).toHaveAttribute("aria-selected", "true");
    await user.keyboard("{ArrowRight}");
    expect(tabs[0]).toHaveFocus();
    await user.keyboard("{ArrowLeft}");
    expect(tabs[3]).toHaveFocus();
    await user.keyboard("{Home}");
    expect(tabs[0]).toHaveFocus();
    await user.keyboard("{End}");
    expect(tabs[3]).toHaveFocus();
  });

  it("calls onChange on click but not for disabled tabs", async () => {
    const user = userEvent.setup();
    const onChange = vi.fn();
    render(<Tabs items={ITEMS} value="spec" onChange={onChange} />);
    await user.click(screen.getByRole("tab", { name: "Plan" }));
    expect(onChange).toHaveBeenCalledWith("plan");
    await user.click(screen.getByRole("tab", { name: "Tasks" }));
    expect(onChange).toHaveBeenCalledTimes(1);
  });

  it("falls back to the label as the key", async () => {
    const user = userEvent.setup();
    const onChange = vi.fn();
    render(<Tabs items={[{ label: "One" }, { label: "Two" }]} value="One" onChange={onChange} />);
    await user.click(screen.getByRole("tab", { name: "Two" }));
    expect(onChange).toHaveBeenCalledWith("Two");
  });

  it("draws the card type with a tray and a white selected tab", () => {
    render(<Controlled type="card" />);
    const tray = screen.getByRole("tablist").getAttribute("style") ?? "";
    expect(tray).toContain("background: var(--neutral-50)");
    expect(tray).toContain("padding: 4px");
    expect(screen.getByRole("tab", { name: "Spec" }).getAttribute("style")).toContain(
      "background: rgb(255, 255, 255)",
    );
  });

  it("draws the line type with a primary underline on the selected tab", () => {
    render(<Controlled />);
    expect(screen.getByRole("tab", { name: "Spec" }).getAttribute("style")).toContain(
      "2px solid var(--primary-600)",
    );
  });

  it("scrolls horizontally when the tabs overflow", () => {
    render(<Controlled />);
    expect(screen.getByRole("tablist").getAttribute("style")).toContain("overflow-x: auto");
  });

  it("closes a closable tab without selecting it", async () => {
    const user = userEvent.setup();
    const onClose = vi.fn();
    const onChange = vi.fn();
    render(
      <Tabs
        type="card"
        items={[{ label: "A", value: "a", closable: true }]}
        value="b"
        onChange={onChange}
        onClose={onClose}
      />,
    );
    await user.click(screen.getByRole("button", { name: "Close A" }));
    expect(onClose).toHaveBeenCalledWith("a");
    expect(onChange).not.toHaveBeenCalled();
  });
});
