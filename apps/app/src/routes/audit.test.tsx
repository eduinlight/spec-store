import { act, screen, waitFor, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { afterEach } from "vitest";
import { json, mockApi } from "~/test/api";
import { connectLive, FakeWebSocket } from "~/test/fake-socket";
import { acme, me, projectDetail } from "~/test/fixtures";
import { renderAt } from "~/test/render";

afterEach(() => {
  vi.unstubAllGlobals();
  vi.restoreAllMocks();
  FakeWebSocket.instances = [];
});

const entry = (id: number, overrides: Record<string, unknown> = {}) => ({
  id,
  actor: "alice",
  organization: "acme",
  project: "acme/demo",
  action: "document.put",
  target: `001/spec-${id}`,
  created_at: "2026-10-22T09:00:00.000Z",
  client: "Claude Code",
  origin: "198.51.100.4",
  result: "ok",
  detail: null,
  ...overrides,
});

const facets = {
  actors: [
    { value: "alice", count: 2 },
    { value: "rita", count: 1 },
  ],
  clients: [{ value: "Claude Code", count: 2 }],
  actions: [{ value: "document.put", count: 3 }],
  results: [
    { value: "ok", count: 1 },
    { value: "denied", count: 1 },
    { value: "conflict", count: 1 },
  ],
};

const defaultEntries = () => [
  entry(3),
  entry(2, { result: "conflict", detail: "expected r12, current r13", client: null, origin: null }),
  entry(1, { result: "denied", actor: "rita" }),
];

type Table = (query: URLSearchParams) => unknown;

const table =
  (entries: () => unknown[] = defaultEntries, total = 3): Table =>
  (query) => ({
    entries: entries(),
    total,
    page: Number(query.get("page") ?? 1),
    page_size: Number(query.get("page_size") ?? 25),
    facets,
  });

function setup(handler: Table = table(), extra: Record<string, () => unknown> = {}) {
  return mockApi({
    "GET /api/v1/me": () => me,
    "GET /api/v1/orgs": () => ({ organizations: [acme] }),
    "GET /api/v1/orgs/{org}": () => ({ ...acme, created_at: "2026-10-01T10:00:00.000Z" }),
    "GET /api/v1/orgs/{org}/projects": () => ({ projects: [] }),
    "GET /api/v1/projects/{org}/{project}": () => projectDetail(),
    "GET /api/v1/projects/{org}/{project}/audit/table": (request) =>
      handler(new URL(request.url).searchParams),
    "GET /api/v1/orgs/{org}/audit/table": (request) => handler(new URL(request.url).searchParams),
    ...extra,
  });
}

const tableCalls = (api: ReturnType<typeof setup>) =>
  api.calls.filter((call) => call.path.includes("/audit/table"));

const lastQuery = (api: ReturnType<typeof setup>) => {
  const calls = tableCalls(api);
  const call = calls[calls.length - 1];
  return new URL(call?.path ?? "", "http://app.test").searchParams;
};

const dataRows = () => screen.getAllByRole("row").slice(1);

test("the project audit is a table with the design header, pills and footer", async () => {
  setup();
  renderAt("/p/acme/demo/audit");
  expect(await screen.findByRole("heading", { name: "Audit trail" })).toBeInTheDocument();
  expect(screen.getByText("Immutable · append-only")).toBeInTheDocument();
  for (const name of ["Time", "Actor", "Via", "Action", "Target", "Result", "Origin"]) {
    expect(screen.getByRole("columnheader", { name })).toBeInTheDocument();
  }
  const rows = dataRows();
  expect(rows).toHaveLength(3);
  expect(within(rows[0] as HTMLElement).getByText("OK")).toBeInTheDocument();
  expect(within(rows[1] as HTMLElement).getByText("Conflict")).toBeInTheDocument();
  expect(within(rows[1] as HTMLElement).getByText("expected r12, current r13")).toBeInTheDocument();
  expect(within(rows[2] as HTMLElement).getByText("Denied")).toBeInTheDocument();
  expect(within(rows[0] as HTMLElement).getByText("Claude Code")).toBeInTheDocument();
  expect(within(rows[0] as HTMLElement).getByText("198.51.100.4")).toBeInTheDocument();
  expect(screen.getByText("0 of 3 row(s) selected.")).toBeInTheDocument();
  expect(screen.getByText("Page 1 of 1")).toBeInTheDocument();
  expect(screen.getByRole("button", { name: "Export" })).toBeInTheDocument();
  expect(screen.queryByRole("button", { name: "Reset" })).not.toBeInTheDocument();
});

test("the organization audit uses the same table with a project column", async () => {
  const api = setup();
  renderAt("/o/acme/audit");
  expect(await screen.findByRole("columnheader", { name: "Project" })).toBeInTheDocument();
  expect(screen.getAllByText("acme/demo")).not.toHaveLength(0);
  expect(api.calls.some((call) => call.path.startsWith("/api/v1/orgs/acme/audit/table"))).toBe(
    true,
  );
});

test("filters, sort, page and page size come from the url", async () => {
  const api = setup(table(undefined, 120));
  renderAt(
    "/p/acme/demo/audit?q=spec&actor=alice&actor=rita&result=denied&period=week&sort=oldest&page=2&page_size=50",
  );
  await screen.findByRole("heading", { name: "Audit trail" });
  const query = lastQuery(api);
  expect(query.get("q")).toBe("spec");
  expect(query.getAll("actor")).toEqual(["alice", "rita"]);
  expect(query.getAll("result")).toEqual(["denied"]);
  expect(query.get("period")).toBe("week");
  expect(query.get("sort")).toBe("oldest");
  expect(query.get("page")).toBe("2");
  expect(query.get("page_size")).toBe("50");
  expect(screen.getByText("Page 2 of 3")).toBeInTheDocument();
  expect(screen.getByRole("textbox", { name: "Filter the audit trail" })).toHaveValue("spec");
  expect(screen.getByRole("button", { name: "Reset" })).toBeInTheDocument();
});

test("choosing a facet value filters, returns to page one and shows counts", async () => {
  const api = setup(table(undefined, 120));
  const { router } = renderAt("/p/acme/demo/audit?page=3");
  await screen.findByRole("heading", { name: "Audit trail" });
  await userEvent.click(screen.getByRole("button", { name: "Result" }));
  const dialog = screen.getByRole("dialog", { name: "Result" });
  expect(within(dialog).getByText("Denied")).toBeInTheDocument();
  await userEvent.click(within(dialog).getByRole("checkbox", { name: /Denied/ }));
  await waitFor(() => expect(lastQuery(api).getAll("result")).toEqual(["denied"]));
  expect(lastQuery(api).get("page")).toBe("1");
  expect(router.state.location.search).toBe("?result=denied");
});

test("the period facet holds a single value", async () => {
  const api = setup();
  renderAt("/p/acme/demo/audit");
  await screen.findByRole("heading", { name: "Audit trail" });
  await userEvent.click(screen.getByRole("button", { name: "Period" }));
  await userEvent.click(screen.getByRole("checkbox", { name: /Last 7 days/ }));
  await waitFor(() => expect(lastQuery(api).get("period")).toBe("week"));
  await userEvent.click(screen.getByRole("checkbox", { name: /Last 24 hours/ }));
  await waitFor(() => expect(lastQuery(api).get("period")).toBe("day"));
  await userEvent.click(screen.getByRole("checkbox", { name: /Last 24 hours/ }));
  await waitFor(() => expect(lastQuery(api).get("period")).toBe("all"));
});

test("the text filter waits for a pause and keeps what was typed", async () => {
  const api = setup();
  const { router } = renderAt("/p/acme/demo/audit");
  const input = await screen.findByRole("textbox", { name: "Filter the audit trail" });
  expect(input).toHaveAttribute("placeholder", "Filter target, client, origin…");
  const before = tableCalls(api).length;
  await userEvent.type(input, "docs");
  expect(tableCalls(api)).toHaveLength(before);
  await waitFor(() => expect(lastQuery(api).get("q")).toBe("docs"));
  expect(tableCalls(api)).toHaveLength(before + 1);
  expect(input).toHaveValue("docs");
  expect(router.state.location.search).toBe("?q=docs");
});

test("reset clears every filter and the text", async () => {
  const api = setup();
  const { router } = renderAt("/p/acme/demo/audit?q=spec&result=denied&sort=oldest");
  await userEvent.click(await screen.findByRole("button", { name: "Reset" }));
  await waitFor(() => expect(router.state.location.search).toBe("?sort=oldest"));
  expect(screen.getByRole("textbox", { name: "Filter the audit trail" })).toHaveValue("");
  expect(screen.queryByRole("button", { name: "Reset" })).not.toBeInTheDocument();
  expect(lastQuery(api).get("q")).toBeNull();
});

test("the Time header toggles the sort", async () => {
  const api = setup();
  renderAt("/p/acme/demo/audit");
  const header = await screen.findByRole("columnheader", { name: "Time" });
  expect(header).toHaveAttribute("aria-sort", "descending");
  await userEvent.click(within(header).getByRole("button"));
  await waitFor(() => expect(lastQuery(api).get("sort")).toBe("oldest"));
  await waitFor(() =>
    expect(screen.getByRole("columnheader", { name: "Time" })).toHaveAttribute(
      "aria-sort",
      "ascending",
    ),
  );
});

test("the View menu hides Via, Target and Origin", async () => {
  setup();
  renderAt("/p/acme/demo/audit");
  await screen.findByRole("heading", { name: "Audit trail" });
  await userEvent.click(screen.getByRole("button", { name: "View" }));
  for (const name of ["Via", "Target", "Origin"]) {
    await userEvent.click(screen.getByRole("checkbox", { name }));
  }
  for (const name of ["Via", "Target", "Origin"]) {
    expect(screen.queryByRole("columnheader", { name })).not.toBeInTheDocument();
  }
  expect(screen.queryByText("198.51.100.4")).not.toBeInTheDocument();
  expect(screen.getByRole("columnheader", { name: "Result" })).toBeInTheDocument();
});

test("the pagination buttons drive the page", async () => {
  const api = setup(table(undefined, 120));
  renderAt("/p/acme/demo/audit");
  expect(await screen.findByText("Page 1 of 5")).toBeInTheDocument();
  expect(screen.getByRole("button", { name: "Go to first page" })).toBeDisabled();
  await userEvent.click(screen.getByRole("button", { name: "Go to next page" }));
  await waitFor(() => expect(lastQuery(api).get("page")).toBe("2"));
  await userEvent.click(screen.getByRole("button", { name: "Go to last page" }));
  await waitFor(() => expect(lastQuery(api).get("page")).toBe("5"));
  await userEvent.click(screen.getByRole("button", { name: "Go to previous page" }));
  await waitFor(() => expect(lastQuery(api).get("page")).toBe("4"));
  await userEvent.click(screen.getByRole("button", { name: "Go to first page" }));
  await waitFor(() => expect(lastQuery(api).get("page")).toBe("1"));
}, 20000);

test("rows per page changes the page size and the page count", async () => {
  const api = setup(table(undefined, 120));
  renderAt("/p/acme/demo/audit");
  await screen.findByText("Page 1 of 5");
  await userEvent.click(screen.getByRole("combobox", { name: "Rows per page" }));
  await userEvent.click(screen.getByRole("option", { name: "100" }));
  await waitFor(() => expect(lastQuery(api).get("page_size")).toBe("100"));
  expect(await screen.findByText("Page 1 of 2")).toBeInTheDocument();
});

test("selected rows are counted, exported by id and cleared when the page changes", async () => {
  const clicks: string[] = [];
  vi.spyOn(HTMLAnchorElement.prototype, "click").mockImplementation(function (
    this: HTMLAnchorElement,
  ) {
    clicks.push(this.getAttribute("href") ?? "");
  });
  setup(table(undefined, 120));
  renderAt("/p/acme/demo/audit?result=denied");
  await screen.findByRole("heading", { name: "Audit trail" });
  await userEvent.click(screen.getByRole("button", { name: "Export" }));
  expect(clicks).toEqual(["/api/v1/projects/acme/demo/audit/export.csv?result=denied"]);
  expect(await screen.findByText("Export started.")).toBeInTheDocument();

  const boxes = screen.getAllByRole("checkbox", { name: "Select row" });
  await userEvent.click(boxes[0] as HTMLElement);
  await userEvent.click(boxes[2] as HTMLElement);
  expect(screen.getByText("2 of 3 row(s) selected.")).toBeInTheDocument();
  await userEvent.click(screen.getByRole("button", { name: "Export 2 selected" }));
  expect(clicks[1]).toBe("/api/v1/projects/acme/demo/audit/export.csv?ids=3%2C1");

  await userEvent.click(screen.getByRole("button", { name: "Go to next page" }));
  await waitFor(() => expect(screen.getByText("0 of 3 row(s) selected.")).toBeInTheDocument());
  expect(screen.getByRole("button", { name: "Export" })).toBeInTheDocument();
});

test("select all picks every row of the page", async () => {
  setup();
  renderAt("/o/acme/audit");
  await userEvent.click(
    await screen.findByRole("checkbox", { name: "Select all rows on this page" }),
  );
  expect(screen.getByText("3 of 3 row(s) selected.")).toBeInTheDocument();
  expect(screen.getByRole("button", { name: "Export 3 selected" })).toBeInTheDocument();
  await userEvent.click(screen.getByRole("checkbox", { name: "Select all rows on this page" }));
  expect(screen.getByText("0 of 3 row(s) selected.")).toBeInTheDocument();
});

test("an organization export downloads from the organization endpoint", async () => {
  const clicks: string[] = [];
  vi.spyOn(HTMLAnchorElement.prototype, "click").mockImplementation(function (
    this: HTMLAnchorElement,
  ) {
    clicks.push(this.getAttribute("href") ?? "");
  });
  setup();
  renderAt("/o/acme/audit?q=x");
  await userEvent.click(await screen.findByRole("button", { name: "Export" }));
  expect(clicks).toEqual(["/api/v1/orgs/acme/audit/export.csv?q=x"]);
});

test("an empty result says so", async () => {
  setup(table(() => [], 0));
  renderAt("/p/acme/demo/audit?q=nothing");
  expect(await screen.findByText("No audit entries match.")).toBeInTheDocument();
  expect(screen.getByText("Page 1 of 1")).toBeInTheDocument();
});

test("a person who is not an owner reads the authorization message", async () => {
  setup(() => json({ code: "forbidden", message: "no" }, 403));
  renderAt("/p/acme/demo/audit");
  expect(await screen.findByRole("alert")).toHaveTextContent("You are not allowed to do this.");
});

test("a new entry appears on the project audit without reloading", async () => {
  let entries = [entry(1)];
  setup(table(() => entries, 1));
  renderAt("/p/acme/demo/audit");
  await screen.findByText("001/spec-1");
  const socket = connectLive("acme/demo");
  entries = [entry(2), entry(1)];
  act(() =>
    socket.receive({ project: "acme/demo", audit_id: 2, action: "document.put", result: "ok" }),
  );
  expect(await screen.findByText("001/spec-2")).toBeInTheDocument();
});

test("the screen speaks Spanish", async () => {
  setup();
  renderAt("/p/acme/demo/audit", "es");
  expect(await screen.findByRole("heading", { name: "Registro de auditoría" })).toBeInTheDocument();
  expect(screen.getByText("Inmutable · solo añadir")).toBeInTheDocument();
  expect(screen.getByText("0 de 3 fila(s) seleccionada(s).")).toBeInTheDocument();
  expect(screen.getByText("Página 1 de 1")).toBeInTheDocument();
  expect(screen.getByRole("button", { name: "Exportar" })).toBeInTheDocument();
  expect(screen.getByText("Denegado")).toBeInTheDocument();
  expect(
    screen.getByRole("textbox", { name: "Filtrar el registro de auditoría" }),
  ).toBeInTheDocument();
});
