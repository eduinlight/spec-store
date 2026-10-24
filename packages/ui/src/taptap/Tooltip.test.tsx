import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { Floating, Tooltip } from "./Tooltip";

describe("Tooltip", () => {
  it("opens on hover and closes on mouse leave", async () => {
    const user = userEvent.setup();
    render(
      <Tooltip title="Hint">
        <button type="button">Trigger</button>
      </Tooltip>,
    );
    expect(screen.queryByRole("tooltip")).toBeNull();
    await user.hover(screen.getByRole("button"));
    expect(screen.getByRole("tooltip")).toHaveTextContent("Hint");
    await user.unhover(screen.getByRole("button"));
    expect(screen.queryByRole("tooltip")).toBeNull();
  });

  it("opens on keyboard focus and closes on blur and Escape", async () => {
    const user = userEvent.setup();
    render(
      <Tooltip title="Hint">
        <button type="button">Trigger</button>
      </Tooltip>,
    );
    await user.tab();
    expect(screen.getByRole("tooltip")).toBeInTheDocument();
    await user.keyboard("{Escape}");
    expect(screen.queryByRole("tooltip")).toBeNull();
    await user.tab({ shift: true });
    await user.tab();
    expect(screen.getByRole("tooltip")).toBeInTheDocument();
    await user.tab();
    expect(screen.queryByRole("tooltip")).toBeNull();
  });

  it("points the trigger at the bubble while open", async () => {
    const user = userEvent.setup();
    render(
      <Tooltip title="Hint">
        <button type="button">Trigger</button>
      </Tooltip>,
    );
    const trigger = screen.getByRole("button");
    expect(trigger).not.toHaveAttribute("aria-describedby");
    await user.hover(trigger);
    expect(trigger).toHaveAttribute("aria-describedby", screen.getByRole("tooltip").id);
  });

  it("supports the controlled open prop", () => {
    const { rerender } = render(
      <Tooltip title="Hint" open>
        <span>Trigger</span>
      </Tooltip>,
    );
    expect(screen.getByRole("tooltip")).toBeInTheDocument();
    rerender(
      <Tooltip title="Hint" open={false}>
        <span>Trigger</span>
      </Tooltip>,
    );
    expect(screen.queryByRole("tooltip")).toBeNull();
  });

  it("lets long titles wrap inside a 260px bubble", () => {
    render(
      <Tooltip title={"A very long title ".repeat(10)} open>
        <span>Trigger</span>
      </Tooltip>,
    );
    const bubble = screen.getByRole("tooltip");
    const style = bubble.getAttribute("style") ?? "";
    for (const declaration of [
      "background: rgb(0, 0, 0)",
      "color: rgb(255, 255, 255)",
      "padding: 8px",
      "border-radius: 4px",
      "white-space: normal",
      "width: max-content",
      "max-width: 260px",
      "text-align: left",
      "font: 400 12px/18px var(--font-sans)",
    ]) {
      expect(style).toContain(declaration);
    }
  });

  it("does not render an empty title", () => {
    render(
      <Tooltip title="" open>
        <span>Trigger</span>
      </Tooltip>,
    );
    expect(screen.queryByRole("tooltip")).toBeNull();
  });

  it("places the bubble on each side", () => {
    const sides = {
      top: { bottom: "100%" },
      bottom: { top: "100%" },
      left: { right: "100%" },
      right: { left: "100%" },
    } as const;
    for (const [placement, expected] of Object.entries(sides)) {
      const { container, unmount } = render(
        <Floating placement={placement as keyof typeof sides} color="#000">
          Panel
        </Floating>,
      );
      const [property, value] = Object.entries(expected)[0] as [string, string];
      expect(container.firstElementChild?.getAttribute("style")).toContain(`${property}: ${value}`);
      unmount();
    }
  });
});
