import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { FacetedFilter, type FacetedFilterOption } from "./FacetedFilter";

const options: FacetedFilterOption[] = [
  { value: "create", label: "spec.create", count: 12, mono: true },
  { value: "update", label: "spec.update", count: 8 },
  { value: "delete", label: "spec.delete", count: 3 },
  { value: "approve", label: "task.approve", count: 5 },
];

const trigger = (name: string | RegExp = /Action/) => screen.getByRole("button", { name });

describe("FacetedFilter", () => {
  it("renders a dashed trigger named after the label", () => {
    render(<FacetedFilter label="Action" options={options} />);
    expect(trigger("Action")).toHaveAttribute("aria-expanded", "false");
    expect(trigger("Action").style.height).toBe("36px");
    expect(trigger("Action").style.border).toBe("1px dashed var(--neutral-400)");
  });

  it("opens a panel with a searchable list of checkboxes and counts", async () => {
    render(<FacetedFilter label="Action" options={options} searchPlaceholder="Search action" />);
    await userEvent.click(trigger());
    expect(trigger()).toHaveAttribute("aria-expanded", "true");
    expect(screen.getByPlaceholderText("Search action")).toHaveFocus();
    expect(screen.getAllByRole("checkbox")).toHaveLength(4);
    expect(
      screen.getByRole("checkbox", { name: /spec.create/ }).closest("label"),
    ).toHaveTextContent("12");
  });

  it("filters the options case-insensitively", async () => {
    render(<FacetedFilter label="Action" options={options} searchPlaceholder="Search action" />);
    await userEvent.click(trigger());
    await userEvent.type(screen.getByPlaceholderText("Search action"), "TASK");
    expect(screen.getAllByRole("checkbox")).toHaveLength(1);
    expect(screen.getByRole("checkbox", { name: /task.approve/ })).toBeInTheDocument();
  });

  it("shows the empty text when nothing matches", async () => {
    render(<FacetedFilter label="Action" options={options} searchPlaceholder="Search action" />);
    await userEvent.click(trigger());
    await userEvent.type(screen.getByPlaceholderText("Search action"), "nothing");
    expect(screen.queryAllByRole("checkbox")).toHaveLength(0);
    expect(screen.getByText("No results found.")).toBeInTheDocument();
  });

  it("toggles options, reports the selection and names the trigger with the count", async () => {
    const onChange = vi.fn();
    render(<FacetedFilter label="Action" options={options} onChange={onChange} />);
    await userEvent.click(trigger("Action"));
    await userEvent.click(screen.getByRole("checkbox", { name: /spec.update/ }));
    expect(onChange).toHaveBeenLastCalledWith(["update"]);
    expect(trigger("Action, 1 selected")).toBeInTheDocument();
    expect(screen.getByRole("checkbox", { name: /spec.update/ })).toBeChecked();
    await userEvent.click(screen.getByRole("checkbox", { name: /spec.update/ }));
    expect(onChange).toHaveBeenLastCalledWith([]);
    expect(trigger("Action")).toBeInTheDocument();
  });

  it("shows selected labels as tags up to the inline limit and a count beyond it", async () => {
    render(<FacetedFilter label="Action" options={options} defaultValue={["create", "update"]} />);
    expect(trigger("Action, 2 selected")).toHaveTextContent("spec.create");
    expect(trigger("Action, 2 selected")).toHaveTextContent("spec.update");
    await userEvent.click(trigger("Action, 2 selected"));
    await userEvent.click(screen.getByRole("checkbox", { name: /spec.delete/ }));
    expect(trigger("Action, 3 selected")).toHaveTextContent("3 selected");
    expect(trigger("Action, 3 selected")).not.toHaveTextContent("spec.create");
  });

  it("clears the selection from the footer and hides the footer when empty", async () => {
    const onChange = vi.fn();
    render(
      <FacetedFilter
        label="Action"
        options={options}
        defaultValue={["create"]}
        onChange={onChange}
      />,
    );
    await userEvent.click(trigger("Action, 1 selected"));
    await userEvent.click(screen.getByRole("button", { name: "Clear filters" }));
    expect(onChange).toHaveBeenCalledWith([]);
    expect(screen.queryByRole("button", { name: "Clear filters" })).not.toBeInTheDocument();
    expect(screen.getByRole("checkbox", { name: /spec.create/ })).not.toBeChecked();
  });

  it("supports a controlled value", async () => {
    const onChange = vi.fn();
    render(
      <FacetedFilter label="Action" options={options} value={["delete"]} onChange={onChange} />,
    );
    await userEvent.click(trigger("Action, 1 selected"));
    await userEvent.click(screen.getByRole("checkbox", { name: /spec.create/ }));
    expect(onChange).toHaveBeenCalledWith(["delete", "create"]);
    expect(screen.getByRole("checkbox", { name: /spec.create/ })).not.toBeChecked();
  });

  it("closes with Escape returning focus, and on an outside click", async () => {
    render(
      <>
        <button type="button">outside</button>
        <FacetedFilter label="Action" options={options} />
      </>,
    );
    await userEvent.click(trigger("Action"));
    await userEvent.keyboard("{Escape}");
    expect(screen.queryAllByRole("checkbox")).toHaveLength(0);
    expect(trigger("Action")).toHaveFocus();
    await userEvent.click(trigger("Action"));
    await userEvent.click(screen.getByRole("button", { name: "outside" }));
    expect(screen.queryAllByRole("checkbox")).toHaveLength(0);
  });
});
