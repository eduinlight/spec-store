import { apiQuery, exportUrl, isFiltered, parseView, viewParams } from "./audit-table";

const view = (search: string) => parseView(new URLSearchParams(search));

test("an empty query reads as the defaults", () => {
  expect(view("")).toEqual({
    q: "",
    actors: [],
    clients: [],
    actions: [],
    results: [],
    period: "all",
    sort: "newest",
    page: 1,
    pageSize: 25,
  });
  expect(isFiltered(view(""))).toBe(false);
});

test("repeated parameters become lists and invalid values fall back", () => {
  const parsed = view("actor=a&actor=b&result=denied&period=year&page=0&page_size=7&sort=oldest");
  expect(parsed.actors).toEqual(["a", "b"]);
  expect(parsed.results).toEqual(["denied"]);
  expect(parsed.period).toBe("all");
  expect(parsed.page).toBe(1);
  expect(parsed.pageSize).toBe(25);
  expect(parsed.sort).toBe("oldest");
  expect(isFiltered(parsed)).toBe(true);
});

test("the url keeps only what differs from the defaults", () => {
  expect(viewParams(view("")).toString()).toBe("");
  const search = "q=docs&client=Cursor&period=week&sort=oldest&page=3&page_size=50";
  expect(viewParams(view(search)).toString()).toBe(
    "q=docs&client=Cursor&period=week&sort=oldest&page=3&page_size=50",
  );
});

test("the api query carries the filters and the page", () => {
  expect(apiQuery(view("q=%20x%20&action=a&page=2"))).toMatchObject({
    q: "x",
    action: ["a"],
    page: 2,
    page_size: 25,
    sort: "newest",
  });
});

test("an export uses the ids when rows are selected and the filters otherwise", () => {
  const filtered = view("result=denied&page=4&page_size=10");
  expect(exportUrl("/api/v1/orgs/acme", filtered, [])).toBe(
    "/api/v1/orgs/acme/audit/export.csv?result=denied",
  );
  expect(exportUrl("/api/v1/orgs/acme", filtered, [3, 1])).toBe(
    "/api/v1/orgs/acme/audit/export.csv?ids=3%2C1",
  );
  expect(exportUrl("/api/v1/orgs/acme", view(""), [])).toBe("/api/v1/orgs/acme/audit/export.csv");
});
