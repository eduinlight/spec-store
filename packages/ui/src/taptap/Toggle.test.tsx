import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { Toggle } from "./Toggle";

describe("Toggle", () => {
  it("is a switch that reports aria-checked", () => {
    const { rerender } = render(<Toggle aria-label="Public" />);
    expect(screen.getByRole("switch", { name: "Public" })).toHaveAttribute("aria-checked", "false");
    rerender(<Toggle aria-label="Public" checked />);
    expect(screen.getByRole("switch")).toHaveAttribute("aria-checked", "true");
  });

  it("calls onChange with the next value on click", async () => {
    const onChange = vi.fn();
    render(<Toggle checked onChange={onChange} />);
    await userEvent.click(screen.getByRole("switch"));
    expect(onChange).toHaveBeenCalledWith(false);
  });

  it("toggles with Space and Enter", async () => {
    const onChange = vi.fn();
    render(<Toggle onChange={onChange} />);
    await userEvent.tab();
    expect(screen.getByRole("switch")).toHaveFocus();
    await userEvent.keyboard(" ");
    await userEvent.keyboard("{Enter}");
    expect(onChange).toHaveBeenCalledTimes(2);
    expect(onChange).toHaveBeenCalledWith(true);
  });

  it("stays controlled when checked is given", async () => {
    render(<Toggle checked={false} />);
    await userEvent.click(screen.getByRole("switch"));
    expect(screen.getByRole("switch")).toHaveAttribute("aria-checked", "false");
  });

  it("works uncontrolled with a default", async () => {
    const onChange = vi.fn();
    render(<Toggle defaultChecked onChange={onChange} />);
    await userEvent.click(screen.getByRole("switch"));
    expect(screen.getByRole("switch")).toHaveAttribute("aria-checked", "false");
    expect(onChange).toHaveBeenCalledWith(false);
  });

  it("ignores everything when disabled", async () => {
    const onChange = vi.fn();
    render(<Toggle disabled onChange={onChange} />);
    const toggle = screen.getByRole("switch");
    expect(toggle).toBeDisabled();
    await userEvent.click(toggle);
    toggle.focus();
    await userEvent.keyboard(" ");
    expect(onChange).not.toHaveBeenCalled();
  });

  it("uses the design colours for each state", () => {
    const { rerender } = render(<Toggle />);
    expect(screen.getByRole("switch").style.background).toBe("var(--neutral-400)");
    rerender(<Toggle checked />);
    expect(screen.getByRole("switch").style.background).toBe("var(--primary-600)");
    rerender(<Toggle checked disabled />);
    expect(screen.getByRole("switch").style.background).toBe("var(--primary-400)");
    rerender(<Toggle disabled />);
    expect(screen.getByRole("switch").style.background).toBe("var(--neutral-200)");
  });

  it("is 36 by 20 pixels and lays the knob out per state", () => {
    const { rerender } = render(<Toggle />);
    expect(screen.getByRole("switch")).toHaveStyle({
      width: "36px",
      height: "20px",
      flexDirection: "row-reverse",
    });
    rerender(<Toggle checked />);
    expect(screen.getByRole("switch")).toHaveStyle({ flexDirection: "row" });
  });

  it("shows ON and OFF text with the wider sizes", () => {
    const { rerender } = render(<Toggle showText />);
    expect(screen.getByText("OFF")).toBeInTheDocument();
    expect(screen.getByRole("switch")).toHaveStyle({ width: "50px" });
    rerender(<Toggle showText checked />);
    expect(screen.getByText("ON")).toBeInTheDocument();
    expect(screen.getByRole("switch")).toHaveStyle({ width: "45px" });
  });

  it("shows a focus ring on keyboard focus", async () => {
    render(<Toggle />);
    await userEvent.tab();
    expect(screen.getByRole("switch").style.boxShadow).toContain("var(--focus-ring)");
  });

  it("merges style last", () => {
    render(<Toggle style={{ marginLeft: 4 }} />);
    expect(screen.getByRole("switch")).toHaveStyle({ marginLeft: "4px" });
  });
});
