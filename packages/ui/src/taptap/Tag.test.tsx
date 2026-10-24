import { fireEvent, render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { createRef } from "react";
import { CheckableTag, StatusTag, Tag } from "./Tag";

describe("Tag", () => {
  it("renders the default label and forwards the ref", () => {
    const ref = createRef<HTMLSpanElement>();
    render(<Tag ref={ref} />);
    expect(screen.getByText("Tag")).toBe(ref.current);
  });

  it.each([
    ["basic", "transparent", "var(--neutral-500)", "1px 4px"],
    ["dark", "var(--auxiliary-600)", "rgb(255, 255, 255)", "1px 6px"],
    ["light", "var(--auxiliary-200)", "var(--auxiliary-600)", "1px 6px"],
    ["primary", "var(--primary-100)", "var(--primary-600)", "1px 6px"],
  ] as const)("draws the %s type", (type, background, color, padding) => {
    render(<Tag type={type}>Label</Tag>);
    const tag = screen.getByText("Label");
    expect(tag.style.background).toBe(background);
    expect(tag.style.color).toBe(color);
    expect(tag.style.padding).toBe(padding);
    expect(tag.style.height).toBe("20px");
  });

  it("draws the basic border", () => {
    render(<Tag>Label</Tag>);
    expect(screen.getByText("Label").style.boxShadow).toBe("inset 0 0 0 1px var(--neutral-300)");
  });

  it("applies a custom colour only to the dark type", () => {
    const { rerender } = render(
      <Tag type="dark" color="rgb(1, 2, 3)">
        Label
      </Tag>,
    );
    expect(screen.getByText("Label").style.background).toBe("rgb(1, 2, 3)");
    rerender(
      <Tag type="light" color="rgb(1, 2, 3)">
        Label
      </Tag>,
    );
    expect(screen.getByText("Label").style.background).toBe("var(--auxiliary-200)");
  });

  it("has no close button unless closable", () => {
    render(<Tag>Label</Tag>);
    expect(screen.queryByRole("button")).not.toBeInTheDocument();
  });

  it("renders a labelled close button that calls onClose", async () => {
    const onClose = vi.fn();
    render(
      <Tag closable onClose={onClose}>
        Label
      </Tag>,
    );
    expect(screen.getByText("Label").style.height).toBe("24px");
    const close = screen.getByRole("button", { name: "Remove" });
    expect(close).toHaveAttribute("type", "button");
    await userEvent.click(close);
    await userEvent.keyboard("{Enter}");
    await userEvent.keyboard(" ");
    expect(onClose).toHaveBeenCalledTimes(3);
  });

  it("accepts a custom close label", () => {
    render(
      <Tag closable closeLabel="Remove Alpha">
        Alpha
      </Tag>,
    );
    expect(screen.getByRole("button", { name: "Remove Alpha" })).toBeInTheDocument();
  });

  it("shows the focus ring on the close button", async () => {
    render(<Tag closable>Label</Tag>);
    await userEvent.tab();
    expect(screen.getByRole("button").style.boxShadow).toBe("var(--focus-ring)");
  });
});

describe("CheckableTag", () => {
  it("is a toggle button reflecting checked", () => {
    const { rerender } = render(<CheckableTag>Option</CheckableTag>);
    const button = screen.getByRole("button", { name: "Option" });
    expect(button).toHaveAttribute("type", "button");
    expect(button).toHaveAttribute("aria-pressed", "false");
    rerender(<CheckableTag checked>Option</CheckableTag>);
    expect(screen.getByRole("button")).toHaveAttribute("aria-pressed", "true");
  });

  it("forwards the ref and renders the default label", () => {
    const ref = createRef<HTMLButtonElement>();
    render(<CheckableTag ref={ref} />);
    expect(ref.current).toBe(screen.getByRole("button", { name: "Tag" }));
  });

  it.each([
    ["large", "40px", "16px"],
    ["medium", "36px", "14px"],
    ["small", "24px", "12px"],
  ] as const)("draws the %s size", (size, height, fontSize) => {
    render(<CheckableTag size={size}>Option</CheckableTag>);
    const button = screen.getByRole("button");
    expect(button.style.height).toBe(height);
    expect(button.style.font).toContain(fontSize);
  });

  it("calls onChange with the toggled value on click, Enter and Space", async () => {
    const onChange = vi.fn();
    const { rerender } = render(<CheckableTag onChange={onChange}>Option</CheckableTag>);
    await userEvent.click(screen.getByRole("button"));
    expect(onChange).toHaveBeenLastCalledWith(true);
    rerender(
      <CheckableTag checked onChange={onChange}>
        Option
      </CheckableTag>,
    );
    await userEvent.keyboard("{Enter}");
    expect(onChange).toHaveBeenLastCalledWith(false);
    await userEvent.keyboard(" ");
    expect(onChange).toHaveBeenCalledTimes(3);
  });

  it("draws the checked corner mark", () => {
    const { container, rerender } = render(<CheckableTag>Option</CheckableTag>);
    expect(container.querySelector("button span")).toBeNull();
    rerender(<CheckableTag checked>Option</CheckableTag>);
    const mark = container.querySelector("button span") as HTMLElement;
    expect(mark).toHaveAttribute("aria-hidden", "true");
    expect(mark.style.borderWidth).toBe("0px 0px 18px 18px");
  });

  it("draws default, hover, checked, pressed and focus states", async () => {
    const { rerender } = render(<CheckableTag>Option</CheckableTag>);
    const button = screen.getByRole("button");
    expect(button.style.color).toBe("var(--neutral-600)");
    fireEvent.mouseEnter(button);
    expect(button.style.color).toBe("var(--primary-500)");
    expect(button.style.boxShadow).toBe("inset 0 0 0 1px var(--primary-500)");
    fireEvent.mouseLeave(button);
    rerender(<CheckableTag checked>Option</CheckableTag>);
    expect(button.style.color).toBe("var(--primary-600)");
    expect(button.style.boxShadow).toBe("inset 0 0 0 1px var(--primary-600)");
    await userEvent.tab();
    expect(button.style.boxShadow).toBe("inset 0 0 0 1px var(--primary-600), var(--focus-ring)");
    fireEvent.mouseDown(button);
    expect(button.style.boxShadow).toContain("var(--primary-600)");
  });

  it("disables natively and ignores activation", async () => {
    const onChange = vi.fn();
    render(
      <CheckableTag disabled checked onChange={onChange}>
        Option
      </CheckableTag>,
    );
    const button = screen.getByRole("button");
    expect(button).toBeDisabled();
    expect(button).toHaveAttribute("aria-pressed", "true");
    expect(button.style.background).toBe("var(--neutral-100)");
    expect(button.style.cursor).toBe("not-allowed");
    await userEvent.click(button);
    expect(onChange).not.toHaveBeenCalled();
  });
});

describe("StatusTag", () => {
  it.each([
    ["approved", "Approved", "var(--success-600)", "var(--success-100)"],
    ["underReview", "Under Review", "var(--warning-600)", "var(--warning-100)"],
    ["rejected", "Rejected", "var(--danger-600)", "var(--danger-100)"],
    ["currentVersion", "Current Version", "var(--info-600)", "var(--info-100)"],
    ["cancel", "Cancel", "var(--neutral-400)", "var(--neutral-100)"],
    ["installed", "Installed", "var(--info-600)", "var(--info-100)"],
    ["wishlist", "Wishlist", "var(--warning-600)", "var(--warning-100)"],
  ] as const)("draws %s", (status, label, color, background) => {
    render(<StatusTag status={status} />);
    const tag = screen.getByText(label);
    expect(tag.style.color).toBe(color);
    expect(tag.style.background).toBe(background);
  });

  it("defaults to approved and small", () => {
    render(<StatusTag />);
    const tag = screen.getByText("Approved");
    expect(tag.style.height).toBe("24px");
  });

  it.each([
    ["small", "24px", "16px"],
    ["medium", "28px", "18px"],
    ["large", "32px", "20px"],
  ] as const)("draws the %s size", (size, height, dot) => {
    const { container } = render(<StatusTag size={size} />);
    expect(screen.getByText("Approved").style.height).toBe(height);
    expect((container.querySelector("span span") as HTMLElement).style.width).toBe(dot);
  });

  it("hides the decorative dot from assistive technology", () => {
    const { container } = render(<StatusTag />);
    expect(container.querySelector("span span")).toHaveAttribute("aria-hidden", "true");
  });

  it("shows custom children instead of the label and forwards the ref", () => {
    const ref = createRef<HTMLSpanElement>();
    render(
      <StatusTag ref={ref} status="rejected">
        Blocked
      </StatusTag>,
    );
    expect(screen.getByText("Blocked")).toBe(ref.current);
    expect(screen.queryByText("Rejected")).not.toBeInTheDocument();
  });
});
