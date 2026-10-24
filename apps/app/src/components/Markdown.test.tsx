import { render } from "@testing-library/react";
import { Markdown } from "./Markdown";

function html(body: string): HTMLElement {
  return render(<Markdown body={body} />).container;
}

test("renders headings one level down, lists and tables", () => {
  const view = html("# Title\n\n- one\n- two\n\n| a | b |\n|---|---|\n| 1 | 2 |\n");
  expect(view.querySelector("h1")).toBeNull();
  expect(view.querySelector("h2")?.textContent).toBe("Title");
  expect(view.querySelectorAll("li")).toHaveLength(2);
  expect(view.querySelector("table td:last-child")?.textContent).toBe("2");
});

test("renders task boxes with their state, read-only", () => {
  const view = html("- [x] T01 done\n- [ ] T02 todo\n");
  const boxes = view.querySelectorAll<HTMLInputElement>("input[type=checkbox]");
  expect(boxes).toHaveLength(2);
  expect([...boxes].filter((b) => b.checked)).toHaveLength(1);
  expect([...boxes].every((b) => b.disabled)).toBe(true);
});

test("raw HTML is shown as text and never interpreted", () => {
  const view = html("<script>alert(1)</script>\n\nword <b>bold</b> end\n");
  expect(view.querySelector("script")).toBeNull();
  expect(view.querySelector("b")).toBeNull();
  expect(view.textContent).toContain("<script>alert(1)</script>");
  expect(view.textContent).toContain("<b>bold</b>");
});

test("unsafe link schemes are dropped and safe ones kept", () => {
  const view = html(
    "[bad](javascript:alert(1)) [good](https://example.com/x) [rel](../other.md) [mail](mailto:a@b.c) ![img](data:image/png;base64,AAAA)\n",
  );
  const hrefs = [...view.querySelectorAll("a")].map((a) => a.getAttribute("href") ?? "");
  expect(hrefs.some((h) => h.startsWith("javascript:"))).toBe(false);
  expect(hrefs).toContain("https://example.com/x");
  expect(hrefs).toContain("../other.md");
  expect(hrefs).toContain("mailto:a@b.c");
  expect(view.innerHTML).not.toContain("data:image");
});

test("handles CRLF and code blocks", () => {
  const view = html("# T\r\n\r\npara\r\n\r\n```sh\r\nls -la\r\n```\r\n");
  expect(view.querySelector("h2")?.textContent).toBe("T");
  expect(view.querySelector("p")?.textContent).toBe("para");
  expect(view.querySelector("code.language-sh")?.textContent).toContain("ls -la");
});

test("an empty body renders nothing", () => {
  expect(html("").textContent).toBe("");
});
