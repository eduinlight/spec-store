import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { Checkbox } from "./Checkbox";

describe("Checkbox", () => {
  it("is a native checkbox with its label", () => {
    render(<Checkbox label="Remember me" />);
    const box = screen.getByRole("checkbox", { name: "Remember me" });
    expect(box.tagName).toBe("INPUT");
    expect(box).not.toBeChecked();
  });

  it("reflects checked", () => {
    render(<Checkbox checked label="a" />);
    expect(screen.getByRole("checkbox")).toBeChecked();
  });

  it("reflects indeterminate", () => {
    render(<Checkbox indeterminate label="a" />);
    expect(screen.getByRole("checkbox")).toBePartiallyChecked();
  });

  it("calls onChange with the next value on click and on the label text", async () => {
    const onChange = vi.fn();
    const { rerender } = render(<Checkbox label="Pick" onChange={onChange} />);
    await userEvent.click(screen.getByRole("checkbox"));
    expect(onChange).toHaveBeenLastCalledWith(true);
    rerender(<Checkbox checked label="Pick" onChange={onChange} />);
    await userEvent.click(screen.getByText("Pick"));
    expect(onChange).toHaveBeenLastCalledWith(false);
    expect(onChange).toHaveBeenCalledTimes(2);
  });

  it("toggles with Space", async () => {
    const onChange = vi.fn();
    render(<Checkbox label="Pick" onChange={onChange} />);
    await userEvent.tab();
    expect(screen.getByRole("checkbox")).toHaveFocus();
    await userEvent.keyboard(" ");
    expect(onChange).toHaveBeenCalledWith(true);
  });

  it("stays controlled when checked is given", async () => {
    render(<Checkbox checked={false} />);
    await userEvent.click(screen.getByRole("checkbox"));
    expect(screen.getByRole("checkbox")).not.toBeChecked();
  });

  it("works uncontrolled with defaultChecked", async () => {
    render(<Checkbox defaultChecked />);
    expect(screen.getByRole("checkbox")).toBeChecked();
    await userEvent.click(screen.getByRole("checkbox"));
    expect(screen.getByRole("checkbox")).not.toBeChecked();
  });

  it("ignores everything when disabled", async () => {
    const onChange = vi.fn();
    render(<Checkbox disabled label="Pick" onChange={onChange} />);
    const box = screen.getByRole("checkbox");
    expect(box).toBeDisabled();
    await userEvent.click(box);
    await userEvent.click(screen.getByText("Pick"));
    expect(onChange).not.toHaveBeenCalled();
  });

  it("draws the box colours for each state", () => {
    const { container, rerender } = render(<Checkbox />);
    const mark = () => container.querySelector("label > span") as HTMLElement;
    expect(mark()).toHaveStyle({ width: "16px", height: "16px" });
    expect(mark().style.background).toBe("rgb(255, 255, 255)");
    expect(mark().style.boxShadow).toContain("var(--neutral-300)");
    rerender(<Checkbox checked />);
    expect(mark().style.background).toBe("var(--primary-600)");
    expect(container.querySelector("svg")).toBeInTheDocument();
    rerender(<Checkbox indeterminate />);
    expect(mark().style.background).toBe("var(--primary-600)");
    expect(container.querySelector("svg")).not.toBeInTheDocument();
    rerender(<Checkbox disabled />);
    expect(mark().style.background).toBe("var(--neutral-100)");
  });

  it("highlights the border on hover and shows a ring on focus", async () => {
    const { container } = render(<Checkbox label="Pick" />);
    const mark = container.querySelector("label > span") as HTMLElement;
    await userEvent.hover(screen.getByText("Pick"));
    expect(mark.style.boxShadow).toContain("var(--primary-500)");
    await userEvent.tab();
    expect(mark.style.boxShadow).toContain("var(--focus-ring)");
  });

  it("merges style on the label", () => {
    const { container } = render(<Checkbox style={{ marginTop: 3 }} />);
    expect(container.querySelector("label")).toHaveStyle({ marginTop: "3px" });
  });
});
