import { en } from "./en";
import { es } from "./es";

const leaves = (value: unknown, prefix = ""): [string, unknown][] => {
  if (typeof value !== "object" || value === null) return [[prefix, value]];
  return Object.entries(value).flatMap(([key, child]) =>
    leaves(child, prefix ? `${prefix}.${key}` : key),
  );
};

test("English and Spanish landing copy have exactly the same keys", () => {
  expect(leaves(es).map(([key]) => key)).toEqual(leaves(en).map(([key]) => key));
});

test.each([
  ["English", en],
  ["Spanish", es],
])("%s landing copy has no empty string", (_language, copy) => {
  const entries = leaves(copy);
  expect(entries.length).toBeGreaterThan(100);
  for (const [key, value] of entries) {
    expect(typeof value, key).toBe("string");
    expect((value as string).trim(), key).not.toBe("");
  }
});
