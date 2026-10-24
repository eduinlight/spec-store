import { headingsOf, slugify } from "./markdown";

test("headings of level two and three become contents entries with stable ids", () => {
  const body = "# Title\n\n## Summary\n\ntext\n\n### Sub **part**\n\n#### Deeper\n\n## Summary\n";
  expect(headingsOf(body)).toEqual([
    { level: 2, text: "Summary", id: "summary" },
    { level: 3, text: "Sub part", id: "sub-part" },
  ]);
});

test("headings inside code fences are ignored", () => {
  expect(headingsOf("## Real\n\n```\n## Fake\n```\n")).toEqual([
    { level: 2, text: "Real", id: "real" },
  ]);
});

test("slugs keep letters and digits of any language", () => {
  expect(slugify("Requisitos funcionales (v2)")).toBe("requisitos-funcionales-v2");
  expect(slugify("Cobertura de acción")).toBe("cobertura-de-acción");
});
