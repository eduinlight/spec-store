import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { Select } from "./Select";

const options = [
  { label: "Reader", value: "reader" },
  { label: "Writer", value: "writer" },
  { label: "Locked", value: "locked", disabled: true },
  { label: "Owner", value: "owner" },
];

const trigger = () => screen.getByRole("combobox");

describe("Select", () => {
  it("renders a closed combobox with the placeholder", () => {
    render(<Select options={options} />);
    expect(trigger()).toHaveAttribute("aria-expanded", "false");
    expect(trigger()).toHaveAttribute("aria-haspopup", "listbox");
    expect(trigger()).toHaveTextContent("Select");
    expect(screen.queryByRole("listbox")).not.toBeInTheDocument();
  });

  it("uses a custom placeholder and width", () => {
    const { container } = render(<Select options={options} placeholder="Pick" width="150px" />);
    expect(trigger()).toHaveTextContent("Pick");
    expect(container.firstElementChild).toHaveStyle({ width: "150px" });
  });

  it.each([
    ["large", "0 12px", 40],
    ["medium", "0 12px", 36],
    ["small", "0 8px", 24],
  ] as const)("keeps the %s trigger padding, gap and height", (size, padding, height) => {
    render(<Select options={options} size={size} value="reader" />);
    expect(trigger()).toHaveStyle({ padding, gap: "8px", height: `${height}px` });
  });

  it("keeps the trigger padding while open and when empty", async () => {
    render(<Select options={options} />);
    expect(trigger()).toHaveStyle({ padding: "0 12px", gap: "8px" });
    await userEvent.click(trigger());
    expect(trigger()).toHaveStyle({ padding: "0 12px", gap: "8px" });
  });

  it("shows the label of the selected value", () => {
    render(<Select options={options} value="writer" />);
    expect(trigger()).toHaveTextContent("Writer");
  });

  it("shows the default value uncontrolled", async () => {
    const onChange = vi.fn();
    render(<Select options={options} defaultValue="reader" onChange={onChange} />);
    expect(trigger()).toHaveTextContent("Reader");
    await userEvent.click(trigger());
    await userEvent.click(screen.getByRole("option", { name: "Owner" }));
    expect(trigger()).toHaveTextContent("Owner");
    expect(onChange).toHaveBeenCalledWith("owner");
    expect(screen.queryByRole("listbox")).not.toBeInTheDocument();
  });

  it("stays controlled when value is given", async () => {
    const onChange = vi.fn();
    render(<Select options={options} value="reader" onChange={onChange} />);
    await userEvent.click(trigger());
    await userEvent.click(screen.getByRole("option", { name: "Owner" }));
    expect(onChange).toHaveBeenCalledWith("owner");
    expect(trigger()).toHaveTextContent("Reader");
  });

  it("opens on click and flags the options", async () => {
    render(<Select options={options} value="writer" />);
    await userEvent.click(trigger());
    expect(trigger()).toHaveAttribute("aria-expanded", "true");
    expect(trigger()).toHaveAttribute("aria-controls", screen.getByRole("listbox").id);
    expect(screen.getAllByRole("option")).toHaveLength(4);
    expect(screen.getByRole("option", { name: "Writer" })).toHaveAttribute("aria-selected", "true");
    expect(screen.getByRole("option", { name: "Reader" })).toHaveAttribute(
      "aria-selected",
      "false",
    );
  });

  it("closes on a second click and on outside click", async () => {
    render(
      <>
        <Select options={options} />
        <button type="button">outside</button>
      </>,
    );
    await userEvent.click(trigger());
    await userEvent.click(trigger());
    expect(screen.queryByRole("listbox")).not.toBeInTheDocument();
    await userEvent.click(trigger());
    await userEvent.click(screen.getByText("outside"));
    expect(screen.queryByRole("listbox")).not.toBeInTheDocument();
  });

  it.each(["{Enter}", " ", "{ArrowDown}"])("opens with %s", async (key) => {
    render(<Select options={options} />);
    await userEvent.tab();
    expect(trigger()).toHaveFocus();
    await userEvent.keyboard(key);
    expect(screen.getByRole("listbox")).toBeInTheDocument();
  });

  it("moves with the arrows, skips disabled options and chooses with Enter", async () => {
    const onChange = vi.fn();
    render(<Select options={options} onChange={onChange} />);
    await userEvent.tab();
    await userEvent.keyboard("{ArrowDown}");
    const active = () => trigger().getAttribute("aria-activedescendant");
    expect(active()).toBe(screen.getByRole("option", { name: "Reader" }).id);
    await userEvent.keyboard("{ArrowDown}{ArrowDown}");
    expect(active()).toBe(screen.getByRole("option", { name: "Owner" }).id);
    await userEvent.keyboard("{ArrowDown}");
    expect(active()).toBe(screen.getByRole("option", { name: "Owner" }).id);
    await userEvent.keyboard("{ArrowUp}");
    expect(active()).toBe(screen.getByRole("option", { name: "Writer" }).id);
    await userEvent.keyboard("{Enter}");
    expect(onChange).toHaveBeenCalledWith("writer");
    expect(screen.queryByRole("listbox")).not.toBeInTheDocument();
    expect(trigger()).toHaveFocus();
  });

  it("starts on the selected option", async () => {
    render(<Select options={options} value="writer" />);
    await userEvent.tab();
    await userEvent.keyboard("{Enter}");
    expect(trigger().getAttribute("aria-activedescendant")).toBe(
      screen.getByRole("option", { name: "Writer" }).id,
    );
  });

  it("closes with Escape", async () => {
    render(<Select options={options} />);
    await userEvent.tab();
    await userEvent.keyboard("{Enter}{Escape}");
    expect(screen.queryByRole("listbox")).not.toBeInTheDocument();
    expect(trigger()).toHaveAttribute("aria-expanded", "false");
  });

  it("does not select disabled options by click", async () => {
    const onChange = vi.fn();
    render(<Select options={options} onChange={onChange} />);
    await userEvent.click(trigger());
    await userEvent.click(screen.getByRole("option", { name: "Locked" }));
    expect(onChange).not.toHaveBeenCalled();
  });

  it("ignores everything when disabled", async () => {
    const onChange = vi.fn();
    render(<Select options={options} disabled onChange={onChange} />);
    expect(trigger()).toHaveAttribute("aria-disabled", "true");
    expect(trigger()).toHaveAttribute("tabindex", "-1");
    await userEvent.click(trigger());
    trigger().focus();
    await userEvent.keyboard("{Enter}{ArrowDown}");
    expect(screen.queryByRole("listbox")).not.toBeInTheDocument();
    expect(trigger()).toHaveStyle({ cursor: "not-allowed" });
    expect(trigger().style.background).toBe("var(--neutral-50)");
  });

  it("draws hover, open and error borders", async () => {
    const { rerender } = render(<Select options={options} />);
    expect(trigger().style.boxShadow).toContain("var(--neutral-300)");
    await userEvent.hover(trigger());
    expect(trigger().style.boxShadow).toContain("var(--primary-500)");
    await userEvent.click(trigger());
    expect(trigger().style.boxShadow).toContain("var(--primary-600)");
    rerender(<Select options={options} error />);
    expect(trigger().style.boxShadow).toContain("var(--danger-600)");
    expect(trigger()).toHaveAttribute("aria-invalid", "true");
  });

  it("honours the open prop", () => {
    render(<Select options={options} open />);
    expect(trigger()).toHaveAttribute("aria-expanded", "true");
    expect(screen.getByRole("listbox")).toBeInTheDocument();
  });

  it("names the trigger", () => {
    render(<Select options={options} aria-label="Role" />);
    expect(screen.getByRole("combobox", { name: "Role" })).toBeInTheDocument();
  });

  describe("multiple", () => {
    it("toggles values, keeps the list open and shows tags", async () => {
      const onChange = vi.fn();
      render(<Select options={options} multiple defaultValue={["reader"]} onChange={onChange} />);
      expect(screen.getByRole("combobox")).toHaveTextContent("Reader");
      await userEvent.click(trigger());
      expect(screen.getByRole("listbox")).toHaveAttribute("aria-multiselectable", "true");
      await userEvent.click(screen.getByRole("option", { name: "Owner" }));
      expect(onChange).toHaveBeenLastCalledWith(["reader", "owner"]);
      expect(screen.getByRole("listbox")).toBeInTheDocument();
      await userEvent.click(screen.getByRole("option", { name: "Reader" }));
      expect(onChange).toHaveBeenLastCalledWith(["owner"]);
      expect(trigger()).toHaveTextContent("Owner");
    });

    it("removes a tag with its close button", async () => {
      const onChange = vi.fn();
      render(<Select options={options} multiple value={["reader", "owner"]} onChange={onChange} />);
      await userEvent.click(screen.getByRole("button", { name: "Remove Reader" }));
      expect(onChange).toHaveBeenCalledWith(["owner"]);
      expect(screen.queryByRole("listbox")).not.toBeInTheDocument();
    });

    it("chooses with Enter and keeps the list open", async () => {
      const onChange = vi.fn();
      render(<Select options={options} multiple onChange={onChange} />);
      await userEvent.tab();
      await userEvent.keyboard("{ArrowDown}{Enter}");
      expect(onChange).toHaveBeenCalledWith(["reader"]);
      expect(screen.getByRole("listbox")).toBeInTheDocument();
    });

    it("uses the tag padding once there is a selection", () => {
      render(<Select options={options} multiple value={["reader"]} />);
      expect(trigger()).toHaveStyle({ padding: "3px 12px 3px 4px" });
    });

    it("shows the placeholder when empty", () => {
      render(<Select options={options} multiple placeholder="Roles" />);
      expect(trigger()).toHaveTextContent("Roles");
    });
  });
});
