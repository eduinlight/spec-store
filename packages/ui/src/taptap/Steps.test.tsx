import { render, screen } from "@testing-library/react";
import { Steps } from "./Steps";

const ITEMS = [
  { title: "Specify", description: "Write the spec" },
  { title: "Plan" },
  { title: "Tasks" },
];

describe("Steps", () => {
  it("renders an ordered list with the current step marked", () => {
    render(<Steps items={ITEMS} current={1} />);
    expect(screen.getByRole("list")).toBeInTheDocument();
    const steps = screen.getAllByRole("listitem");
    expect(steps).toHaveLength(3);
    expect(steps[1]).toHaveAttribute("aria-current", "step");
    expect(steps[0]).not.toHaveAttribute("aria-current");
    expect(steps[2]).not.toHaveAttribute("aria-current");
    expect(screen.getByRole("list").tagName).toBe("OL");
  });

  it("shows descriptions and numbers the pending steps", () => {
    render(<Steps items={ITEMS} current={0} />);
    expect(screen.getByText("Write the spec")).toBeInTheDocument();
    expect(screen.getByText("2")).toBeInTheDocument();
    expect(screen.getByText("3")).toBeInTheDocument();
  });

  it("replaces the number of finished steps with a check", () => {
    const { container } = render(<Steps items={ITEMS} current={2} />);
    expect(screen.queryByText("1")).toBeNull();
    expect(container.querySelectorAll("svg")).toHaveLength(2);
  });

  it("honours a per-item status and the error status of the current step", () => {
    render(<Steps items={ITEMS} current={1} status="error" />);
    const steps = screen.getAllByRole("listitem");
    expect(steps[1]?.querySelector("span")?.getAttribute("style")).toContain(
      "background: var(--danger-100)",
    );
  });

  it("lays out vertically", () => {
    render(<Steps items={ITEMS} current={1} direction="vertical" />);
    expect(screen.getByRole("list").getAttribute("style")).toContain("flex-direction: column");
    expect(screen.getAllByRole("listitem")[1]).toHaveAttribute("aria-current", "step");
  });
});
