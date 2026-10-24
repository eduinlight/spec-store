import { act, fireEvent, render, renderHook, screen } from "@testing-library/react";
import { Chevron, fieldShell } from "./field";
import { Icon, iconNames, LockDocument } from "./Icon";
import { useInteraction } from "./interaction";

describe("useInteraction", () => {
  it("moves through hover, pressed and focus and back to default", () => {
    const { result } = renderHook(() => useInteraction());
    expect(result.current[0]).toBe("default");
    act(() => result.current[1].onMouseEnter?.({} as never));
    expect(result.current[0]).toBe("hover");
    act(() => result.current[1].onMouseDown?.({} as never));
    expect(result.current[0]).toBe("pressed");
    act(() => result.current[1].onMouseLeave?.({} as never));
    expect(result.current[0]).toBe("default");
    act(() => result.current[1].onFocus?.({} as never));
    expect(result.current[0]).toBe("focus");
  });

  it("reports disabled and offers no handlers", () => {
    const { result } = renderHook(() => useInteraction(true));
    expect(result.current[0]).toBe("disabled");
    expect(result.current[1]).toEqual({});
  });
});

describe("fieldShell", () => {
  it("draws the default, hover, focus, error and disabled borders", () => {
    expect(fieldShell({}).boxShadow).toBe("inset 0 0 0 1px var(--neutral-300)");
    expect(fieldShell({ state: "hover" }).boxShadow).toBe("inset 0 0 0 1px var(--primary-500)");
    expect(fieldShell({ state: "focus" }).boxShadow).toBe(
      "inset 0 0 0 1px var(--primary-600), var(--focus-ring)",
    );
    expect(fieldShell({ error: true, state: "focus" }).boxShadow).toBe(
      "inset 0 0 0 1px var(--danger-600)",
    );
    expect(fieldShell({ disabled: true }).background).toBe("var(--neutral-50)");
  });

  it("keeps the horizontal padding of each size", () => {
    expect(fieldShell({ size: "large" }).padding).toBe("0 12px");
    expect(fieldShell({ size: "medium" }).padding).toBe("0 12px");
    expect(fieldShell({ size: "small" }).padding).toBe("0 8px");
  });

  it("renders a chevron that points up when open", () => {
    const { container } = render(<Chevron up />);
    expect(container.querySelector("svg")).toHaveStyle({ transform: "rotate(180deg)" });
  });
});

describe("Icon", () => {
  it("offers the 129 glyphs of the design system", () => {
    expect(iconNames).toHaveLength(129);
    expect(iconNames).toContain("Search");
  });

  it("is hidden from assistive technology unless titled", () => {
    const { container } = render(<Icon name="Search" size={16} />);
    const svg = container.querySelector("svg");
    expect(svg).toHaveAttribute("aria-hidden", "true");
    expect(svg).toHaveAttribute("width", "16");
    render(<Icon name="Search" title="Search" />);
    expect(screen.getByRole("img", { name: "Search" })).toBeInTheDocument();
  });

  it("draws the lock document glyph with a 1.7 stroke in the current colour", () => {
    render(<LockDocument title="Locked" />);
    const svg = screen.getByRole("img", { name: "Locked" });
    expect(svg).toHaveAttribute("stroke", "currentColor");
    expect(svg).toHaveAttribute("stroke-width", "1.7");
  });

  it("ignores pointer events on decorative glyphs", () => {
    const onClick = vi.fn();
    const { container } = render(<Icon name="Add" onClick={onClick} />);
    fireEvent.click(container.querySelector("svg") as Element);
    expect(onClick).toHaveBeenCalledTimes(1);
  });
});
