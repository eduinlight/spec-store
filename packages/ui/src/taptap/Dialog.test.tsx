import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { useState } from "react";
import { Dialog } from "./Dialog";

const Harness = () => {
  const [open, setOpen] = useState(false);
  return (
    <>
      <button type="button" onClick={() => setOpen(true)}>
        Open
      </button>
      <Dialog open={open} onCancel={() => setOpen(false)} okText="Save">
        <input aria-label="Name" />
      </Dialog>
    </>
  );
};

describe("Dialog", () => {
  it("renders nothing when closed", () => {
    render(<Dialog open={false} />);
    expect(screen.queryByRole("dialog")).toBeNull();
  });

  it("is a modal dialog labelled by its title, rendered in a portal", () => {
    const { container } = render(<Dialog title="Delete project" />);
    const dialog = screen.getByRole("dialog", { name: "Delete project" });
    expect(dialog).toHaveAttribute("aria-modal", "true");
    expect(container).not.toContainElement(dialog);
    expect(document.body).toContainElement(dialog);
    expect(dialog).toHaveStyle({ boxShadow: "var(--elevation-4)", width: "400px" });
    expect(dialog.parentElement?.getAttribute("style")).toContain("background: var(--overlay)");
  });

  it("maps size names to widths", () => {
    const { rerender } = render(<Dialog size="medium" />);
    expect(screen.getByRole("dialog")).toHaveStyle({ width: "600px" });
    rerender(<Dialog size="560px" />);
    expect(screen.getByRole("dialog")).toHaveStyle({ width: "560px" });
  });

  it("calls the confirm and cancel handlers", async () => {
    const user = userEvent.setup();
    const onOk = vi.fn();
    const onCancel = vi.fn();
    render(<Dialog okText="Delete" onOk={onOk} onCancel={onCancel} />);
    await user.click(screen.getByRole("button", { name: "Delete" }));
    expect(onOk).toHaveBeenCalledTimes(1);
    await user.click(screen.getByRole("button", { name: "Cancel" }));
    await user.click(screen.getByRole("button", { name: "Close" }));
    expect(onCancel).toHaveBeenCalledTimes(2);
  });

  it("renders a custom footer or none", () => {
    const { rerender } = render(<Dialog footer={<span>Custom</span>} />);
    expect(screen.getByText("Custom")).toBeInTheDocument();
    expect(screen.queryByRole("button", { name: "Confirm" })).toBeNull();
    rerender(<Dialog footer={null} />);
    expect(screen.queryByRole("button", { name: "Cancel" })).toBeNull();
  });

  it("closes on Escape and on overlay click only", async () => {
    const user = userEvent.setup();
    const onCancel = vi.fn();
    render(<Dialog onCancel={onCancel}>Body</Dialog>);
    await user.click(screen.getByText("Body"));
    expect(onCancel).not.toHaveBeenCalled();
    await user.keyboard("{Escape}");
    expect(onCancel).toHaveBeenCalledTimes(1);
    await user.click(document.querySelector('button[tabindex="-1"]') as HTMLElement);
    expect(onCancel).toHaveBeenCalledTimes(2);
  });

  it("moves focus in, traps Tab and returns focus on close", async () => {
    const user = userEvent.setup();
    render(<Harness />);
    const trigger = screen.getByRole("button", { name: "Open" });
    await user.click(trigger);
    const dialog = screen.getByRole("dialog");
    expect(dialog).toContainElement(document.activeElement as HTMLElement);
    expect(document.activeElement).toBe(screen.getByRole("button", { name: "Close" }));
    await user.tab({ shift: true });
    expect(document.activeElement).toBe(screen.getByRole("button", { name: "Save" }));
    await user.tab();
    expect(document.activeElement).toBe(screen.getByRole("button", { name: "Close" }));
    await user.tab();
    await user.tab();
    await user.tab();
    await user.tab();
    expect(dialog).toContainElement(document.activeElement as HTMLElement);
    await user.keyboard("{Escape}");
    expect(screen.queryByRole("dialog")).toBeNull();
    expect(document.activeElement).toBe(trigger);
  });

  it("renders inline without a portal or overlay", () => {
    const { container } = render(<Dialog inline title="Inline" />);
    expect(container).toContainElement(screen.getByRole("dialog"));
  });
});
