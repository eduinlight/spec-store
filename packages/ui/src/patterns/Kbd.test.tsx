import { render, screen } from "@testing-library/react";
import { Kbd } from "./Kbd";

describe("Kbd", () => {
  it("renders a kbd element with the mono chip style", () => {
    render(<Kbd>↵</Kbd>);
    const chip = screen.getByText("↵");
    expect(chip.tagName).toBe("KBD");
    expect(chip.style.height).toBe("20px");
    expect(chip.style.borderRadius).toBe("3px");
    expect(chip.style.boxShadow).toBe("inset 0 0 0 1px var(--neutral-300)");
  });

  it("offers an inverse tone for dark surfaces", () => {
    render(<Kbd tone="inverse">⌘K</Kbd>);
    expect(screen.getByText("⌘K").style.background).toBe("rgba(255, 255, 255, 0.2)");
  });
});
