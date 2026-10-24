import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { SidebarItem } from "./SidebarItem";

describe("SidebarItem", () => {
  it("renders a button that calls onClick", async () => {
    const onClick = vi.fn();
    render(<SidebarItem label="Specs" onClick={onClick} />);
    await userEvent.click(screen.getByRole("button", { name: "Specs" }));
    expect(onClick).toHaveBeenCalledTimes(1);
  });

  it("renders a link when href is given", () => {
    render(<SidebarItem label="Specs" href="/specs" />);
    expect(screen.getByRole("link", { name: "Specs" })).toHaveAttribute("href", "/specs");
  });

  it("marks the selected item as the current page with a fill and weight 500", () => {
    render(<SidebarItem label="Active specs" selected />);
    const item = screen.getByRole("button", { name: "Active specs" });
    expect(item).toHaveAttribute("aria-current", "page");
    expect(item.style.background).toBe("var(--primary-100)");
    expect(item.style.fontWeight).toBe("500");
  });

  it("does not mark an unselected item", () => {
    render(<SidebarItem label="Specs" />);
    const item = screen.getByRole("button", { name: "Specs" });
    expect(item).not.toHaveAttribute("aria-current");
    expect(item.style.background).toBe("transparent");
    expect(item.style.fontWeight).toBe("400");
  });

  it("highlights on hover", async () => {
    render(<SidebarItem label="Specs" />);
    const item = screen.getByRole("button", { name: "Specs" });
    await userEvent.hover(item);
    expect(item.style.background).toBe("var(--neutral-50)");
    await userEvent.unhover(item);
    expect(item.style.background).toBe("transparent");
  });

  it("shows the icon and the badge count in the accessible name", () => {
    render(<SidebarItem label="Active specs" badge={2} icon={<svg data-testid="icon" />} />);
    expect(screen.getByTestId("icon")).toBeInTheDocument();
    expect(screen.getByRole("button", { name: "Active specs 2" })).toBeInTheDocument();
  });

  it("is reachable and activated with the keyboard", async () => {
    const onClick = vi.fn();
    render(<SidebarItem label="Specs" onClick={onClick} />);
    await userEvent.tab();
    expect(screen.getByRole("button", { name: "Specs" })).toHaveFocus();
    await userEvent.keyboard("{Enter}");
    expect(onClick).toHaveBeenCalledTimes(1);
  });
});
