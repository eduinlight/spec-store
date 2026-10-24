import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { Pagination } from "./Pagination";

describe("Pagination", () => {
  it("is a labelled navigation with the current page marked", () => {
    render(<Pagination current={3} total={5} />);
    expect(screen.getByRole("navigation", { name: "Pagination" })).toBeInTheDocument();
    expect(screen.getByRole("button", { name: "Page 3" })).toHaveAttribute("aria-current", "page");
    expect(screen.getByRole("button", { name: "Page 4" })).not.toHaveAttribute("aria-current");
  });

  it("disables previous on the first page and next on the last", () => {
    const { rerender } = render(<Pagination current={1} total={5} />);
    expect(screen.getByRole("button", { name: "Previous page" })).toBeDisabled();
    expect(screen.getByRole("button", { name: "Next page" })).toBeEnabled();
    rerender(<Pagination current={5} total={5} />);
    expect(screen.getByRole("button", { name: "Previous page" })).toBeEnabled();
    expect(screen.getByRole("button", { name: "Next page" })).toBeDisabled();
  });

  it("reports the chosen page", async () => {
    const user = userEvent.setup();
    const onChange = vi.fn();
    render(<Pagination current={2} total={5} onChange={onChange} />);
    await user.click(screen.getByRole("button", { name: "Next page" }));
    expect(onChange).toHaveBeenLastCalledWith(3);
    await user.click(screen.getByRole("button", { name: "Previous page" }));
    expect(onChange).toHaveBeenLastCalledWith(1);
    await user.click(screen.getByRole("button", { name: "Page 5" }));
    expect(onChange).toHaveBeenLastCalledWith(5);
    await user.click(screen.getByRole("button", { name: "Page 2" }));
    expect(onChange).toHaveBeenCalledTimes(3);
  });

  it("collapses long ranges with ellipses", () => {
    render(<Pagination current={10} total={20} />);
    const labels = screen.getAllByRole("button").map((button) => button.getAttribute("aria-label"));
    expect(labels).toEqual([
      "Previous page",
      "Page 1",
      "Page 9",
      "Page 10",
      "Page 11",
      "Page 20",
      "Next page",
    ]);
    expect(screen.getAllByText("···")).toHaveLength(2);
  });

  it("jumps to a typed page on Enter and ignores out of range pages", async () => {
    const user = userEvent.setup();
    const onChange = vi.fn();
    render(<Pagination current={1} total={5} onChange={onChange} />);
    const input = screen.getByRole("textbox", { name: "Go to page" });
    await user.type(input, "4{Enter}");
    expect(onChange).toHaveBeenCalledWith(4);
    await user.clear(input);
    await user.type(input, "9{Enter}");
    expect(onChange).toHaveBeenCalledTimes(1);
  });

  it("hides the jumper on request", () => {
    render(<Pagination current={1} total={5} showJumper={false} />);
    expect(screen.queryByRole("textbox")).toBeNull();
  });

  it("shows the item range and page size when showTotal is set", () => {
    render(<Pagination current={2} total={5} pageSize={10} totalItems={43} showTotal />);
    expect(screen.getByText(/11–20 of 43 items/)).toBeInTheDocument();
    expect(screen.getByText(/10\/page/)).toBeInTheDocument();
  });

  it("renders the simple variant", async () => {
    const user = userEvent.setup();
    const onChange = vi.fn();
    render(<Pagination simple current={2} total={5} onChange={onChange} />);
    expect(screen.getByRole("navigation", { name: "Pagination" })).toBeInTheDocument();
    expect(screen.getByText("2")).toBeInTheDocument();
    await user.click(screen.getByRole("button", { name: "Next page" }));
    expect(onChange).toHaveBeenCalledWith(3);
  });
});
