import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { DropdownMenu } from "./DropdownMenu";

const items = [
  { label: "One", value: "one" },
  { divider: true },
  { label: "Two", value: "two", extra: "x" },
  { label: "Three", disabled: true },
];

describe("DropdownMenu", () => {
  it("renders a listbox of options with aria-selected", () => {
    render(<DropdownMenu items={items} value="two" />);
    expect(screen.getByRole("listbox")).toBeInTheDocument();
    expect(screen.getAllByRole("option")).toHaveLength(3);
    expect(screen.getByRole("option", { name: /Two/ })).toHaveAttribute("aria-selected", "true");
    expect(screen.getByRole("option", { name: "One" })).toHaveAttribute("aria-selected", "false");
  });

  it("falls back to the label as the key", async () => {
    const onSelect = vi.fn();
    render(<DropdownMenu items={items} onSelect={onSelect} />);
    await userEvent.click(screen.getByRole("option", { name: "One" }));
    expect(onSelect).toHaveBeenCalledWith("one");
  });

  it("does not select disabled items", async () => {
    const onSelect = vi.fn();
    render(<DropdownMenu items={items} onSelect={onSelect} />);
    await userEvent.click(screen.getByRole("option", { name: "Three" }));
    expect(onSelect).not.toHaveBeenCalled();
    expect(screen.getByRole("option", { name: "Three" })).toHaveAttribute("aria-disabled", "true");
  });

  it("marks multiple selection and shows checkboxes", () => {
    render(<DropdownMenu items={items} multiple value={["one", "two"]} />);
    expect(screen.getByRole("listbox")).toHaveAttribute("aria-multiselectable", "true");
    expect(screen.getAllByRole("option", { selected: true })).toHaveLength(2);
  });

  it.each([
    ["large", "6px 8px", "16px"],
    ["medium", "5px 8px", "14px"],
    ["small", "1px 8px", "12px"],
  ] as const)("sizes the %s options", (size, padding, fontSize) => {
    render(<DropdownMenu items={items} size={size} />);
    const option = screen.getByRole("option", { name: "One" });
    expect(option).toHaveStyle({ padding });
    expect(option.style.font).toContain(fontSize);
  });

  it("highlights hover and the active option", async () => {
    render(<DropdownMenu items={items} activeIndex={2} idPrefix="m" />);
    expect(screen.getByRole("option", { name: /Two/ })).toHaveAttribute("id", "m-option-2");
    expect(screen.getByRole("option", { name: /Two/ }).style.background).toBe("var(--neutral-50)");
    await userEvent.hover(screen.getByRole("option", { name: "One" }));
    expect(screen.getByRole("option", { name: "One" }).style.background).toBe("var(--neutral-50)");
  });

  it("colours the selected option", () => {
    render(<DropdownMenu items={items} value="one" />);
    const option = screen.getByRole("option", { name: "One" });
    expect(option.style.background).toBe("var(--primary-100)");
    expect(option.style.color).toBe("var(--primary-600)");
  });
});
