import { fireEvent, render, screen } from "@testing-library/react";
import { Alert } from "./Alert";

describe("Alert", () => {
  it("is an alert for errors and warnings", () => {
    const { rerender } = render(<Alert status="error">Broken</Alert>);
    expect(screen.getByRole("alert")).toHaveTextContent("Broken");
    rerender(<Alert status="warning">Careful</Alert>);
    expect(screen.getByRole("alert")).toBeInTheDocument();
  });

  it("is a status otherwise", () => {
    render(<Alert status="success">Saved</Alert>);
    expect(screen.getByRole("status")).toHaveTextContent("Saved");
    expect(screen.queryByRole("alert")).toBeNull();
  });

  it("shows the title and the tone", () => {
    render(
      <Alert status="info" title="Heads up">
        Body
      </Alert>,
    );
    expect(screen.getByText("Heads up")).toBeInTheDocument();
    expect(screen.getByRole("status").getAttribute("style")).toContain(
      "background: var(--info-100)",
    );
  });

  it("closes through a labelled button", () => {
    const onClose = vi.fn();
    render(<Alert onClose={onClose}>Text</Alert>);
    fireEvent.click(screen.getByRole("button", { name: "Close" }));
    expect(onClose).toHaveBeenCalledTimes(1);
  });

  it("can hide the close button and render an action", () => {
    render(
      <Alert closable={false} action={<a href="#more">More</a>}>
        Text
      </Alert>,
    );
    expect(screen.queryByRole("button")).toBeNull();
    expect(screen.getByRole("link", { name: "More" })).toBeInTheDocument();
  });
});
