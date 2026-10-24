import { fireEvent, render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { createRef } from "react";
import { Button, IconButton } from "./Button";
import { Icon } from "./Icon";

describe("Button", () => {
  it("renders a native button of type button with the default label", () => {
    render(<Button />);
    const button = screen.getByRole("button", { name: "Text" });
    expect(button.tagName).toBe("BUTTON");
    expect(button).toHaveAttribute("type", "button");
  });

  it("allows the submit type and forwards native props and ref", () => {
    const ref = createRef<HTMLButtonElement>();
    render(
      <Button ref={ref} type="submit" data-testid="save" aria-label="Save it">
        Save
      </Button>,
    );
    expect(ref.current).toBe(screen.getByTestId("save"));
    expect(screen.getByRole("button", { name: "Save it" })).toHaveAttribute("type", "submit");
  });

  it.each([
    ["large", "40px", "90px", "16px"],
    ["medium", "36px", "82px", "14px"],
    ["small", "24px", "54px", "12px"],
  ] as const)("draws the %s size", (size, height, minWidth, fontSize) => {
    render(<Button size={size}>Go</Button>);
    const button = screen.getByRole("button");
    expect(button.style.height).toBe(height);
    expect(button.style.minWidth).toBe(minWidth);
    expect(button.style.fontSize).toBe(fontSize);
  });

  it("defaults to medium", () => {
    render(<Button>Go</Button>);
    expect(screen.getByRole("button").style.height).toBe("36px");
  });

  it.each([
    ["primary", "var(--primary-600)", "rgb(255, 255, 255)"],
    ["outline", "rgb(255, 255, 255)", "var(--neutral-600)"],
    ["ghost", "transparent", "var(--neutral-600)"],
    ["link", "transparent", "var(--primary-600)"],
  ] as const)("draws the %s variant", (variant, background, color) => {
    render(<Button variant={variant}>Go</Button>);
    const button = screen.getByRole("button");
    expect(button.style.background).toBe(background);
    expect(button.style.color).toBe(color);
  });

  it("draws the outline border and removes padding for links", () => {
    const { rerender } = render(<Button variant="outline">Go</Button>);
    expect(screen.getByRole("button").style.boxShadow).toBe("inset 0 0 0 1px var(--neutral-300)");
    rerender(<Button variant="link">Go</Button>);
    expect(screen.getByRole("button").style.padding).toBe("0px");
    expect(screen.getByRole("button").style.minWidth).toBe("");
  });

  it("uses danger colours", () => {
    const { rerender } = render(
      <Button danger variant="primary">
        Go
      </Button>,
    );
    expect(screen.getByRole("button").style.background).toBe("var(--danger-600)");
    rerender(
      <Button danger variant="outline">
        Go
      </Button>,
    );
    expect(screen.getByRole("button").style.boxShadow).toBe("inset 0 0 0 1px var(--danger-600)");
  });

  it("fills the width in block mode", () => {
    render(<Button block>Go</Button>);
    const button = screen.getByRole("button");
    expect(button.style.display).toBe("flex");
    expect(button.style.width).toBe("100%");
  });

  it("renders left and right icons", () => {
    render(
      <Button leftIcon={<i data-testid="left" />} rightIcon={<i data-testid="right" />}>
        Go
      </Button>,
    );
    expect(screen.getByTestId("left")).toBeInTheDocument();
    expect(screen.getByTestId("right")).toBeInTheDocument();
  });

  it("merges the style prop last", () => {
    render(<Button style={{ height: 50 }}>Go</Button>);
    expect(screen.getByRole("button").style.height).toBe("50px");
  });

  it("shows hover, pressed and focus states", async () => {
    const onFocus = vi.fn();
    render(
      <Button onFocus={onFocus} variant="primary">
        Go
      </Button>,
    );
    const button = screen.getByRole("button");
    fireEvent.mouseEnter(button);
    expect(button.style.background).toBe("var(--primary-500)");
    fireEvent.mouseDown(button);
    expect(button.style.background).toBe("var(--primary-700)");
    fireEvent.mouseUp(button);
    fireEvent.mouseLeave(button);
    expect(button.style.background).toBe("var(--primary-600)");
    await userEvent.tab();
    expect(button).toHaveFocus();
    expect(onFocus).toHaveBeenCalledTimes(1);
    expect(button.style.boxShadow).toContain("var(--focus-ring)");
    await userEvent.tab();
    expect(button.style.boxShadow).not.toContain("var(--focus-ring)");
  });

  it("keeps the border and adds the ring on focus for outline", async () => {
    render(<Button variant="outline">Go</Button>);
    await userEvent.tab();
    expect(screen.getByRole("button").style.boxShadow).toBe(
      "inset 0 0 0 1px var(--neutral-300), var(--focus-ring)",
    );
  });

  it("disables natively and ignores activation", async () => {
    const onClick = vi.fn();
    render(
      <Button disabled onClick={onClick}>
        Go
      </Button>,
    );
    const button = screen.getByRole("button");
    expect(button).toBeDisabled();
    expect(button.style.background).toBe("var(--primary-300)");
    expect(button.style.cursor).toBe("not-allowed");
    await userEvent.click(button);
    await userEvent.type(button, "{Enter}");
    expect(onClick).not.toHaveBeenCalled();
  });

  it("activates on click, Enter and Space", async () => {
    const onClick = vi.fn();
    render(<Button onClick={onClick}>Go</Button>);
    await userEvent.click(screen.getByRole("button"));
    expect(onClick).toHaveBeenCalledTimes(1);
    await userEvent.keyboard("{Enter}");
    expect(onClick).toHaveBeenCalledTimes(2);
    await userEvent.keyboard(" ");
    expect(onClick).toHaveBeenCalledTimes(3);
  });
});

describe("IconButton", () => {
  const icon = <Icon name="Search" size={16} />;

  it("renders a named native button", () => {
    render(<IconButton icon={icon} aria-label="Search" />);
    const button = screen.getByRole("button", { name: "Search" });
    expect(button).toHaveAttribute("type", "button");
  });

  it("forwards the ref", () => {
    const ref = createRef<HTMLButtonElement>();
    render(<IconButton ref={ref} icon={icon} aria-label="Search" />);
    expect(ref.current).toBe(screen.getByRole("button"));
  });

  it.each([
    ["large", "40px"],
    ["medium", "36px"],
    ["small", "24px"],
  ] as const)("draws the %s size", (size, box) => {
    render(<IconButton icon={icon} size={size} aria-label="Search" />);
    const button = screen.getByRole("button");
    expect(button.style.width).toBe(box);
    expect(button.style.height).toBe(box);
  });

  it("is a circle by default and square on request", () => {
    const { rerender } = render(<IconButton icon={icon} aria-label="Search" />);
    expect(screen.getByRole("button").style.borderRadius).toBe("100px");
    rerender(<IconButton icon={icon} shape="square" aria-label="Search" />);
    expect(screen.getByRole("button").style.borderRadius).toBe("4px");
  });

  it.each([
    ["primary", "var(--primary-600)"],
    ["outline", "rgb(255, 255, 255)"],
    ["ghost", "transparent"],
  ] as const)("draws the %s variant", (variant, background) => {
    render(<IconButton icon={icon} variant={variant} aria-label="Search" />);
    expect(screen.getByRole("button").style.background).toBe(background);
  });

  it("shows hover, pressed and focus states", async () => {
    render(<IconButton icon={icon} aria-label="Search" />);
    const button = screen.getByRole("button");
    fireEvent.mouseEnter(button);
    expect(button.style.background).toBe("var(--primary-500)");
    fireEvent.mouseDown(button);
    expect(button.style.background).toBe("var(--primary-700)");
    fireEvent.mouseLeave(button);
    await userEvent.tab();
    expect(button.style.boxShadow).toContain("var(--focus-ring)");
  });

  it("disables natively and ignores activation", async () => {
    const onClick = vi.fn();
    render(<IconButton icon={icon} disabled onClick={onClick} aria-label="Search" />);
    const button = screen.getByRole("button");
    expect(button).toBeDisabled();
    expect(button.style.background).toBe("var(--primary-300)");
    await userEvent.click(button);
    expect(onClick).not.toHaveBeenCalled();
  });

  it("activates on click, Enter and Space", async () => {
    const onClick = vi.fn();
    render(<IconButton icon={icon} onClick={onClick} aria-label="Search" />);
    await userEvent.click(screen.getByRole("button"));
    await userEvent.keyboard("{Enter}");
    await userEvent.keyboard(" ");
    expect(onClick).toHaveBeenCalledTimes(3);
  });
});
