import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { createRef } from "react";
import { Input } from "./Input";

describe("Input", () => {
  it("renders a real input with the default placeholder", () => {
    render(<Input />);
    expect(screen.getByRole("textbox")).toHaveAttribute("placeholder", "Text input");
  });

  it("forwards the ref to the input element", () => {
    const ref = createRef<HTMLInputElement>();
    render(<Input ref={ref} />);
    expect(ref.current).toBe(screen.getByRole("textbox"));
  });

  it("passes native attributes to the input", () => {
    render(<Input aria-label="Slug" name="slug" type="text" />);
    expect(screen.getByRole("textbox", { name: "Slug" })).toHaveAttribute("name", "slug");
  });

  it.each([
    ["large", 40, "16px"],
    ["medium", 36, "14px"],
    ["small", 24, "12px"],
  ] as const)("sizes the %s field", (size, height, fontSize) => {
    render(<Input size={size} />);
    const field = screen.getByRole("textbox").closest("label");
    expect(field).toHaveStyle({ height: `${height}px` });
    expect(field?.style.font).toContain(fontSize);
  });

  it("works uncontrolled with a default value and reports the new text", async () => {
    const onChange = vi.fn();
    render(<Input defaultValue="ab" onChange={onChange} />);
    const input = screen.getByRole("textbox");
    expect(input).toHaveValue("ab");
    await userEvent.type(input, "c");
    expect(input).toHaveValue("abc");
    expect(onChange).toHaveBeenLastCalledWith("abc");
  });

  it("stays controlled when a value is given", async () => {
    const onChange = vi.fn();
    render(<Input value="fixed" onChange={onChange} />);
    const input = screen.getByRole("textbox");
    await userEvent.type(input, "x");
    expect(input).toHaveValue("fixed");
    expect(onChange).toHaveBeenCalledWith("fixedx");
  });

  it("draws the focus, hover, error and disabled borders", async () => {
    render(<Input />);
    const input = screen.getByRole("textbox");
    const field = input.closest("label") as HTMLElement;
    expect(field.style.boxShadow).toContain("var(--neutral-300)");
    await userEvent.hover(field);
    expect(field.style.boxShadow).toContain("var(--primary-500)");
    await userEvent.click(input);
    expect(field.style.boxShadow).toContain("var(--primary-600)");
  });

  it("draws the error border", () => {
    render(<Input error />);
    expect(screen.getByRole("textbox").closest("label")?.style.boxShadow).toContain(
      "var(--danger-600)",
    );
  });

  it("ignores typing when disabled", async () => {
    const onChange = vi.fn();
    render(<Input disabled onChange={onChange} />);
    const input = screen.getByRole("textbox");
    expect(input).toBeDisabled();
    await userEvent.type(input, "x");
    expect(onChange).not.toHaveBeenCalled();
    expect(input.closest("label")?.style.background).toBe("var(--neutral-50)");
  });

  it("shows prefix and suffix", () => {
    render(<Input prefix={<i>pre</i>} suffix={<i>suf</i>} />);
    expect(screen.getByText("pre")).toBeInTheDocument();
    expect(screen.getByText("suf")).toBeInTheDocument();
  });

  it("clears the value with allowClear and only when there is text", async () => {
    const onChange = vi.fn();
    render(<Input allowClear defaultValue="abc" onChange={onChange} />);
    await userEvent.click(screen.getByRole("button", { name: "Clear" }));
    expect(screen.getByRole("textbox")).toHaveValue("");
    expect(onChange).toHaveBeenCalledWith("");
    expect(screen.queryByRole("button", { name: "Clear" })).not.toBeInTheDocument();
  });

  it("hides the clear button when disabled", () => {
    render(<Input allowClear disabled defaultValue="abc" />);
    expect(screen.queryByRole("button", { name: "Clear" })).not.toBeInTheDocument();
  });

  it("applies style to the field and inputStyle to the input", () => {
    render(<Input style={{ width: 280 }} inputStyle={{ fontFamily: "monospace" }} />);
    const input = screen.getByRole("textbox");
    expect(input.closest("label")).toHaveStyle({ width: "280px" });
    expect(input).toHaveStyle({ fontFamily: "monospace" });
  });
});
