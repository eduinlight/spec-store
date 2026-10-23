import { en } from "./en";
import { es } from "./es";

function keys(value: unknown, prefix = ""): string[] {
  if (typeof value !== "object" || value === null) return [prefix];
  return Object.entries(value).flatMap(([key, child]) =>
    keys(child, prefix ? `${prefix}.${key}` : key),
  );
}

test("English and Spanish have exactly the same keys", () => {
  expect(keys(es).sort()).toEqual(keys(en).sort());
});

test("no translation is empty", () => {
  for (const [language, resource] of [
    ["en", en],
    ["es", es],
  ] as const) {
    for (const key of keys(resource)) {
      const value = key
        .split(".")
        .reduce<unknown>((node, part) => (node as Record<string, unknown>)[part], resource);
      expect(typeof value === "string" && value.trim().length > 0, `${language}:${key}`).toBe(true);
    }
  }
});
