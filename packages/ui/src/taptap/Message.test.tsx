import { act, render, screen } from "@testing-library/react";
import { Message, MessageHost, message } from "./Message";

describe("Message", () => {
  it("renders a status with its text", () => {
    render(<Message status="success">Done</Message>);
    expect(screen.getByRole("status")).toHaveTextContent("Done");
  });
});

describe("MessageHost", () => {
  beforeEach(() => {
    vi.useFakeTimers();
  });

  afterEach(() => {
    vi.useRealTimers();
  });

  it("shows a toast in a polite live region and removes it after 3 seconds", () => {
    render(<MessageHost />);
    const region = screen.getByRole("status", { name: "Notifications" });
    expect(region).toHaveAttribute("aria-live", "polite");
    act(() => {
      message.success("Saved");
    });
    expect(region).toHaveTextContent("Saved");
    act(() => {
      vi.advanceTimersByTime(2900);
    });
    expect(region).toHaveTextContent("Saved");
    act(() => {
      vi.advanceTimersByTime(200);
    });
    expect(region).not.toHaveTextContent("Saved");
  });

  it("stacks toasts of every kind and honours a custom duration", () => {
    render(<MessageHost />);
    const region = screen.getByRole("status", { name: "Notifications" });
    act(() => {
      message.error("Bad");
      message.info("FYI");
      message.warning("Careful", { duration: 500 });
    });
    expect(region).toHaveTextContent("BadFYICareful");
    act(() => {
      vi.advanceTimersByTime(600);
    });
    expect(region).not.toHaveTextContent("Careful");
    expect(region).toHaveTextContent("Bad");
    act(() => {
      vi.advanceTimersByTime(3000);
    });
    expect(region).toBeEmptyDOMElement();
  });
});
