import { en } from "./en";
import { es } from "./es";

function keys(value: unknown, prefix = ""): string[] {
  if (typeof value !== "object" || value === null) return [prefix];
  return Object.entries(value).flatMap(([key, child]) =>
    keys(child, prefix ? `${prefix}.${key}` : key),
  );
}

test("English and Spanish landing copy have the same keys", () => {
  expect(keys(es).sort()).toEqual(keys(en).sort());
});
