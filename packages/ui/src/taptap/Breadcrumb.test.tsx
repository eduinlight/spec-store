import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { Breadcrumb } from "./Breadcrumb";

describe("Breadcrumb", () => {
  it("is a labelled navigation with an ordered list", () => {
    render(<Breadcrumb items={[{ label: "Home", href: "/" }, { label: "Projects" }]} />);
    expect(screen.getByRole("navigation", { name: "Breadcrumb" })).toBeInTheDocument();
    expect(screen.getByRole("list").tagName).toBe("OL");
    expect(screen.getAllByRole("listitem")).toHaveLength(2);
  });

  it("marks the last item as the current page and renders it as text", () => {
    render(<Breadcrumb items={[{ label: "Home", href: "/" }, { label: "Projects" }]} />);
    const current = screen.getByText("Projects");
    expect(current).toHaveAttribute("aria-current", "page");
    expect(screen.queryByRole("link", { name: "Projects" })).toBeNull();
  });

  it("renders items with an href as links", () => {
    render(<Breadcrumb items={[{ label: "Home", href: "/home" }, { label: "Here" }]} />);
    expect(screen.getByRole("link", { name: "Home" })).toHaveAttribute("href", "/home");
  });

  it("renders the other items as buttons and reports navigation", async () => {
    const user = userEvent.setup();
    const onNavigate = vi.fn();
    const onClick = vi.fn();
    const items = [{ label: "Home", onClick }, { label: "Org" }, { label: "Here" }];
    render(<Breadcrumb items={items} onNavigate={onNavigate} />);
    await user.click(screen.getByRole("button", { name: "Home" }));
    expect(onClick).toHaveBeenCalledTimes(1);
    expect(onNavigate).toHaveBeenCalledWith(items[0], 0);
    await user.click(screen.getByRole("button", { name: "Org" }));
    expect(onNavigate).toHaveBeenCalledWith(items[1], 1);
  });

  it("collapses the middle into an ellipsis when over maxItems", () => {
    const items = ["A", "B", "C", "D", "E"].map((label) => ({ label }));
    render(<Breadcrumb items={items} maxItems={3} />);
    expect(screen.getAllByRole("listitem")).toHaveLength(4);
    expect(screen.getByText("···")).toBeInTheDocument();
    expect(screen.queryByText("B")).toBeNull();
    expect(screen.getByText("E")).toHaveAttribute("aria-current", "page");
    expect(screen.queryByRole("button", { name: "···" })).toBeNull();
  });
});
