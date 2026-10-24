import { readFileSync } from "node:fs";
import { resolve } from "node:path";

const tokens = readFileSync(resolve(__dirname, "tokens.css"), "utf8");

function declarations(css: string): Map<string, string> {
  const result = new Map<string, string>();
  for (const [, name, value] of css.matchAll(/(--[a-z0-9-]+)\s*:\s*([^;]+);/gi)) {
    if (name && value) {
      result.set(name, value.replace(/\s+/g, " ").trim());
    }
  }
  return result;
}

const declared = declarations(tokens);

const handoff: Record<string, string> = {
  "--primary-600": "#15C5CE",
  "--primary-500": "#47CFD6",
  "--primary-700": "#00ABB6",
  "--primary-100": "#EEFCFC",
  "--neutral-700": "#1F1F1F",
  "--neutral-600": "#4B4B4B",
  "--neutral-500": "#8E8E8E",
  "--neutral-300": "#E1E1E1",
  "--neutral-200": "#EEEEEE",
  "--neutral-100": "#F5F5F5",
  "--neutral-50": "#FAFAFA",
  "--danger-600": "#F64C4C",
  "--warning-700": "#FE9B0E",
  "--success-700": "#0C9D61",
  "--info-700": "#3A70E2",
  "--auxiliary-600": "#FF8156",
  "--chart-1": "#4887F6",
  "--fs-h1": "30px",
  "--lh-h1": "38px",
  "--fs-body": "14px",
  "--fs-caption": "12px",
  "--radius-sm": "4px",
  "--radius-xs": "2px",
  "--radius-pill": "100px",
  "--header-h": "60px",
  "--sidebar-w": "220px",
  "--control-h-md": "36px",
};

describe("TapTap tokens", () => {
  it.each(Object.entries(handoff))("%s keeps the handoff value %s", (name, value) => {
    expect(declared.get(name)?.toLowerCase()).toBe(value.toLowerCase());
  });

  it("declares the four elevations and the focus ring", () => {
    for (const name of [
      "--elevation-1",
      "--elevation-2",
      "--elevation-3",
      "--elevation-4",
      "--focus-ring",
    ]) {
      expect(declared.has(name)).toBe(true);
    }
  });

  it.each([
    ["--primary", "var(--primary-600)"],
    ["--background", "var(--surface-page)"],
    ["--foreground", "var(--text-strong)"],
    ["--border", "var(--neutral-300)"],
    ["--destructive", "var(--danger-600)"],
    ["--radius", "var(--radius-sm)"],
  ])("aliases %s to %s", (name, value) => {
    expect(declared.get(name)?.toLowerCase()).toBe(value.toLowerCase());
  });

  it("uses the system sans stack and the monospace stack of the handoff", () => {
    expect(declared.get("--font-sans")).toMatch(/^-apple-system, ?BlinkMacSystemFont/);
    expect(declared.get("--font-mono")).toMatch(/^Consolas, ?"SFMono-Regular"/);
  });
});
