import { render, screen } from "@testing-library/react";
import { createRef } from "react";
import { Avatar, AvatarGroup } from "./Avatar";

describe("Avatar", () => {
  it("shows initials with the person name as accessible name when there is no image", () => {
    render(<Avatar name="Ada Lovelace" />);
    const avatar = screen.getByRole("img", { name: "Ada Lovelace" });
    expect(avatar).toHaveTextContent("AL");
    expect(avatar.style.background).toBe("var(--primary-200)");
  });

  it("limits initials to two letters and upper-cases them", () => {
    render(<Avatar name="jean claude van damme" />);
    expect(screen.getByRole("img")).toHaveTextContent("JC");
  });

  it("shows one initial for a single name and none for an empty name", () => {
    const { rerender } = render(<Avatar name="cher" />);
    expect(screen.getByRole("img")).toHaveTextContent("C");
    rerender(<Avatar />);
    expect(screen.getByRole("img")).toHaveTextContent("");
  });

  it("renders the image with the name as alt text", () => {
    render(<Avatar name="Ada Lovelace" src="/ada.png" />);
    const image = screen.getByRole("img", { name: "Ada Lovelace" });
    expect(image.tagName).toBe("IMG");
    expect(image).toHaveAttribute("src", "/ada.png");
    expect(image.parentElement?.style.background).toBe("var(--neutral-100)");
  });

  it("forwards the ref and native props", () => {
    const ref = createRef<HTMLSpanElement>();
    render(<Avatar ref={ref} name="Ada" data-testid="avatar" />);
    expect(ref.current).toBe(screen.getByTestId("avatar"));
  });

  it("uses 32px circle by default and supports other sizes and the square shape", () => {
    const { rerender } = render(<Avatar name="Ada" />);
    let avatar = screen.getByRole("img");
    expect(avatar.style.width).toBe("32px");
    expect(avatar.style.borderRadius).toBe("50%");
    expect(avatar.style.font).toContain("13px");
    rerender(<Avatar name="Ada" size={48} shape="square" />);
    avatar = screen.getByRole("img");
    expect(avatar.style.height).toBe("48px");
    expect(avatar.style.borderRadius).toBe("4px");
    expect(avatar.style.font).toContain("19px");
  });

  it.each([
    ["default", "none", "1"],
    ["hover", "0 0 0 2px var(--primary-500)", "1"],
    ["selected", "0 0 0 2px var(--primary-600)", "1"],
    ["disabled", "none", "0.4"],
  ] as const)("draws the %s state", (state, shadow, opacity) => {
    render(<Avatar name="Ada" state={state} />);
    const avatar = screen.getByRole("img");
    expect(avatar.style.boxShadow).toBe(shadow);
    expect(avatar.style.opacity).toBe(opacity);
  });
});

describe("AvatarGroup", () => {
  it("shows up to max avatars with overlap", () => {
    render(
      <AvatarGroup srcs={["/a.png", "/b.png", "/c.png", "/d.png"]} names={["A", "B", "C", "D"]} />,
    );
    const images = screen.getAllByRole("img", { name: /^[A-D]$/ });
    expect(images).toHaveLength(3);
    expect(images[0]?.parentElement?.style.marginLeft).toBe("0px");
    expect(images[1]?.parentElement?.style.marginLeft).toBe("-8px");
    expect(images[1]?.parentElement?.style.width).toBe("28px");
  });

  it("shows a +N overflow with an accessible label", () => {
    render(<AvatarGroup srcs={["/a.png", "/b.png", "/c.png", "/d.png", "/e.png"]} />);
    const overflow = screen.getByRole("img", { name: "2 more" });
    expect(overflow).toHaveTextContent("+2");
  });

  it("shows no overflow when everything fits", () => {
    render(<AvatarGroup srcs={["/a.png", "/b.png"]} />);
    expect(screen.queryByText(/^\+/)).not.toBeInTheDocument();
  });

  it("honours max and size", () => {
    render(<AvatarGroup srcs={["/a.png", "/b.png", "/c.png"]} max={1} size={40} />);
    const overflow = screen.getByRole("img", { name: "2 more" });
    expect(overflow).toHaveTextContent("+2");
    expect(overflow.style.width).toBe("40px");
  });

  it("is a labelled group and forwards the ref", () => {
    const ref = createRef<HTMLSpanElement>();
    render(<AvatarGroup ref={ref} srcs={["/a.png"]} aria-label="Reviewers" />);
    expect(screen.getByRole("group", { name: "Reviewers" })).toBe(ref.current);
  });
});
