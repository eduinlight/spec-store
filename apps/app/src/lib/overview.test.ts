import { attentionPath, chartData, coveragePercent, periodOf } from "./overview";

test("only 7, 14 and 30 days are periods and 7 is the default", () => {
  expect(periodOf("14")).toBe(14);
  expect(periodOf("30")).toBe(30);
  expect(periodOf("9")).toBe(7);
  expect(periodOf(null)).toBe(7);
});

test("the chart has one series per client ordered by total and one value per day", () => {
  const data = chartData([
    { day: "2026-10-22", by_client: [{ client: "Cursor", count: 1 }] },
    {
      day: "2026-10-23",
      by_client: [
        { client: "Claude Code", count: 4 },
        { client: "Cursor", count: 2 },
      ],
    },
    { day: "2026-10-24", by_client: [] },
  ]);
  expect(data.days).toEqual(["2026-10-22", "2026-10-23", "2026-10-24"]);
  expect(data.series).toEqual([
    { name: "Claude Code", data: [0, 4, 0] },
    { name: "Cursor", data: [1, 2, 0] },
  ]);
  expect(data.total).toBe(7);
  expect(data.max).toBe(10);
});

test("an empty period has a minimum scale of five", () => {
  const data = chartData([{ day: "2026-10-24", by_client: [] }]);
  expect(data.series).toEqual([]);
  expect(data.max).toBe(5);
  expect(data.total).toBe(0);
});

test("coverage is a rounded percentage and absent without a total", () => {
  expect(coveragePercent({ spec_number: 4, covered: 2, total: 3 })).toBe(67);
  expect(coveragePercent({ spec_number: 4, covered: 0, total: 0 })).toBeNull();
  expect(coveragePercent(null)).toBeNull();
});

test("each attention kind opens the right screen of its spec", () => {
  const item = (kind: string) => ({ kind, spec_number: 4, spec_slug: "x", count: 1, detail: [] });
  const base = "/p/acme/demo";
  expect(attentionPath(base, item("approval_waiting"))).toBe("/p/acme/demo/specs/4");
  expect(attentionPath(base, item("open_questions"))).toBe("/p/acme/demo/specs/4/markers");
  expect(attentionPath(base, item("requirement_without_work"))).toBe(
    "/p/acme/demo/specs/4/traceability",
  );
  expect(attentionPath(base, item("unknown"))).toBeNull();
});
