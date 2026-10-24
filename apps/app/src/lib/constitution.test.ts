import { parseConstitution, toRoman } from "./constitution";

const BODY = `# Constitution — Payments

Non-negotiable principles.
They prevail.

1. **The spec rules.** No implementation without an approved spec;
   when a decision is not covered, work stops.
2. **Tests as a gate** Tests are written first.

10. **Language.** Everything is in English.

Version 3.0 · Ratified 2026-10-17
`;

test("the title, the introduction and the numbered principles are read from the markdown", () => {
  expect(parseConstitution(BODY)).toEqual({
    title: "Constitution — Payments",
    intro: "Non-negotiable principles. They prevail.",
    principles: [
      {
        number: 1,
        title: "The spec rules",
        body: "No implementation without an approved spec; when a decision is not covered, work stops.",
      },
      { number: 2, title: "Tests as a gate", body: "Tests are written first." },
      { number: 10, title: "Language", body: "Everything is in English." },
    ],
  });
});

test("a document without numbered principles has none", () => {
  expect(parseConstitution("# Constitution\n\nVersion 1.0\n")).toEqual({
    title: "Constitution",
    intro: "Version 1.0",
    principles: [],
  });
});

test("principle numbers are shown as roman numerals", () => {
  expect([1, 4, 7, 9, 10, 14, 40].map(toRoman)).toEqual(["I", "IV", "VII", "IX", "X", "XIV", "XL"]);
});
