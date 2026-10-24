import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { useState } from "react";
import { Drawer } from "./Drawer";

const Harness = () => {
  const [open, setOpen] = useState(false);
  return (
    <>
      <button type="button" onClick={() => setOpen(true)}>
        Open
      </button>
      <Drawer open={open} onClose={() => setOpen(false)}>
        <input aria-label="Filter" />
      </Drawer>
    </>
  );
};

describe("Drawer", () => {
  it("renders nothing when closed", () => {
    render(<Drawer open={false} />);
    expect(screen.queryByRole("dialog")).toBeNull();
  });

  it("is a modal dialog in a portal, sliding from the right with a width", () => {
    const { container } = render(<Drawer title="Notifications" width={420} />);
    const drawer = screen.getByRole("dialog", { name: "Notifications" });
    expect(drawer).toHaveAttribute("aria-modal", "true");
    expect(container).not.toContainElement(drawer);
    expect(drawer).toHaveStyle({ width: "420px", boxShadow: "var(--elevation-4)" });
    expect(drawer.parentElement).toHaveStyle({ justifyContent: "flex-end" });
    expect(drawer.parentElement?.getAttribute("style")).toContain("background: var(--overlay)");
  });

  it("can slide from the left", () => {
    render(<Drawer placement="left" />);
    expect(screen.getByRole("dialog").parentElement).toHaveStyle({ justifyContent: "flex-start" });
  });

  it("calls the handlers and supports footers", async () => {
    const user = userEvent.setup();
    const onOk = vi.fn();
    const onClose = vi.fn();
    const { rerender } = render(<Drawer onOk={onOk} onClose={onClose} okText="Apply" />);
    await user.click(screen.getByRole("button", { name: "Apply" }));
    await user.click(screen.getByRole("button", { name: "Cancel" }));
    await user.click(screen.getByRole("button", { name: "Close" }));
    expect(onOk).toHaveBeenCalledTimes(1);
    expect(onClose).toHaveBeenCalledTimes(2);
    rerender(<Drawer footer={null} />);
    expect(screen.queryByRole("button", { name: "Cancel" })).toBeNull();
  });

  it("closes on Escape and overlay click", async () => {
    const user = userEvent.setup();
    const onClose = vi.fn();
    render(<Drawer onClose={onClose}>Body</Drawer>);
    await user.click(screen.getByText("Body"));
    expect(onClose).not.toHaveBeenCalled();
    await user.keyboard("{Escape}");
    await user.click(document.querySelector('button[tabindex="-1"]') as HTMLElement);
    expect(onClose).toHaveBeenCalledTimes(2);
  });

  it("moves focus in, traps Tab and returns focus", async () => {
    const user = userEvent.setup();
    render(<Harness />);
    const trigger = screen.getByRole("button", { name: "Open" });
    await user.click(trigger);
    const drawer = screen.getByRole("dialog");
    expect(drawer).toContainElement(document.activeElement as HTMLElement);
    for (let index = 0; index < 6; index += 1) {
      await user.tab();
      expect(drawer).toContainElement(document.activeElement as HTMLElement);
    }
    await user.tab({ shift: true });
    expect(drawer).toContainElement(document.activeElement as HTMLElement);
    await user.keyboard("{Escape}");
    expect(screen.queryByRole("dialog")).toBeNull();
    expect(document.activeElement).toBe(trigger);
  });
});
