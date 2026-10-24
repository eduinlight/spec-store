import { isValidSlug, slugify } from "./slug";

test("proposes a slug from a name", () => {
  expect(slugify("Payments service")).toBe("payments-service");
  expect(slugify("  Café  Añejo!! ")).toBe("cafe-anejo");
  expect(slugify("One two three four five")).toBe("one-two-three-four");
  expect(slugify("***")).toBe("");
});

test("accepts one to four lowercase words joined by hyphens", () => {
  expect(isValidSlug("payments-service")).toBe(true);
  expect(isValidSlug("a-b-c-d")).toBe(true);
  expect(isValidSlug("a-b-c-d-e")).toBe(false);
  expect(isValidSlug("Payments")).toBe(false);
  expect(isValidSlug("a--b")).toBe(false);
  expect(isValidSlug("")).toBe(false);
});
