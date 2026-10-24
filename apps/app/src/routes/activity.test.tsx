import { act, screen, waitFor, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { afterEach } from "vitest";
import { json, mockApi } from "~/test/api";
import { connectLive, FakeWebSocket, lastSocket } from "~/test/fake-socket";
import { acme, activityRow, me, projectDetail } from "~/test/fixtures";
import { renderAt } from "~/test/render";

afterEach(() => {
  vi.unstubAllGlobals();
  FakeWebSocket.instances = [];
});

interface Page {
  rows: unknown[];
  has_more: boolean;
}

function setup(pages: (query: URLSearchParams) => Page) {
  return mockApi({
    "GET /api/v1/me": () => me,
    "GET /api/v1/orgs": () => ({ organizations: [acme] }),
    "GET /api/v1/orgs/{org}/projects": () => ({ projects: [] }),
    "GET /api/v1/projects/{org}/{project}": () => projectDetail(),
    "GET /api/v1/projects/{org}/{project}/activity": (request) =>
      pages(new URL(request.url).searchParams),
  });
}

const feed = () => screen.findByRole("list", { name: "Activity" });

test("a task finished by an agent reads as a sentence with the task and commit", async () => {
  setup(() => ({
    rows: [
      activityRow(9, {
        actor: "dana",
        client: "Claude Code",
        action: "task.done",
        summary_key: "task.done",
        target: "004/T07@a1b2c3d",
      }),
    ],
    has_more: false,
  }));
  renderAt("/p/acme/demo/activity");
  const row = within(await feed()).getByRole("listitem");
  expect(row).toHaveTextContent("dana marked T07 done · commit a1b2c3d");
  expect(within(row).getByText("via Claude Code")).toBeInTheDocument();
  expect(within(row).getByText("OK")).toBeInTheDocument();
  expect(row.querySelector("time")).toHaveAttribute("datetime", "2026-10-24T09:00:00.000Z");
});

test.each([
  [
    { action: "document.put", summary_key: "document.put", target: "001/spec@14" },
    "wrote 001/spec (r14)",
  ],
  [
    { action: "spec.create", summary_key: "spec.create", target: "005-partial" },
    "created spec 005-partial",
  ],
  [
    { action: "spec.status", summary_key: "spec.status", target: "004:draft→clarified" },
    "moved spec 004 from draft to clarified",
  ],
  [
    { action: "constitution.put", summary_key: "constitution.put", target: "v1.2@30" },
    "wrote the constitution v1.2 (r30)",
  ],
  [
    { action: "member.set", summary_key: "member.set", target: "rita:editor" },
    "set rita as editor",
  ],
  [
    { action: "member.remove", summary_key: "member.remove", target: "rita" },
    "removed rita from the project",
  ],
  [
    { action: "project.archive", summary_key: "project.archive", target: "demo" },
    "archived the project",
  ],
  [
    { action: "project.repo", summary_key: "project.repo", target: "demo cleared" },
    "cleared the repository",
  ],
])("%o is worded as %s", async (overrides, sentence) => {
  setup(() => ({ rows: [activityRow(9, overrides)], has_more: false }));
  renderAt("/p/acme/demo/activity");
  expect(within(await feed()).getByRole("listitem")).toHaveTextContent(`dana ${sentence}`);
});

test("a refused write says what was tried and that it was denied", async () => {
  setup(() => ({
    rows: [
      activityRow(9, {
        actor: "rita",
        result: "denied",
        summary_key: "denied",
        target: "004/spec",
      }),
    ],
    has_more: false,
  }));
  renderAt("/p/acme/demo/activity");
  const row = within(await feed()).getByRole("listitem");
  expect(row).toHaveTextContent("rita tried to write a document 004/spec — denied");
  expect(within(row).getByText("Denied")).toBeInTheDocument();
});

test("a conflict shows the stale base and the current revision", async () => {
  setup(() => ({
    rows: [
      activityRow(9, {
        result: "conflict",
        summary_key: "conflict",
        target: "004/spec",
        detail: "expected r12, current r13",
      }),
    ],
    has_more: false,
  }));
  renderAt("/p/acme/demo/activity");
  const row = within(await feed()).getByRole("listitem");
  expect(row).toHaveTextContent("write on 004/spec based on r12 rejected · current r13");
  expect(within(row).getByText("Conflict")).toBeInTheDocument();
});

test("a conflict followed by a save is shown as re-read, merged and saved with the saving revision", async () => {
  setup(() => ({
    rows: [
      activityRow(10, { target: "004/spec@15" }),
      activityRow(9, {
        result: "conflict",
        summary_key: "conflict_resolved",
        resolved_by_audit_id: 10,
        target: "004/spec",
        detail: "expected r12, current r13",
      }),
    ],
    has_more: false,
  }));
  renderAt("/p/acme/demo/activity");
  const rows = within(await feed()).getAllByRole("listitem");
  expect(rows[1]).toHaveTextContent(
    "write on 004/spec based on r12 rejected · current r13 · re-read and merged · saved (r15)",
  );
});

test("filters ask the server for that kind of activity and keep it in the address", async () => {
  const filters: (string | null)[] = [];
  setup((query) => {
    filters.push(query.get("filter"));
    return { rows: [activityRow(9)], has_more: false };
  });
  const { router } = renderAt("/p/acme/demo/activity");
  const group = await screen.findByRole("group", { name: "Filter activity" });
  expect(within(group).getByRole("button", { name: "All" })).toHaveAttribute(
    "aria-pressed",
    "true",
  );
  await userEvent.click(within(group).getByRole("button", { name: "Security" }));
  await waitFor(() => expect(filters).toContain("security"));
  expect(router.state.location.search).toBe("?filter=security");
  await waitFor(() =>
    expect(screen.getByRole("button", { name: "Security" })).toHaveAttribute(
      "aria-pressed",
      "true",
    ),
  );
});

test("a filter with nothing in it says so", async () => {
  setup((query) => ({
    rows: query.get("filter") === "tasks" ? [] : [activityRow(9)],
    has_more: false,
  }));
  renderAt("/p/acme/demo/activity?filter=tasks");
  expect(await screen.findByText("Nothing matches this filter.")).toBeInTheDocument();
});

test("a project without activity shows an empty state", async () => {
  setup(() => ({ rows: [], has_more: false }));
  renderAt("/p/acme/demo/activity");
  expect(await screen.findByText("No activity yet")).toBeInTheDocument();
});

test("Load more asks for the rows before the oldest one and appends them", async () => {
  const queries: string[] = [];
  setup((query) => {
    queries.push(query.toString());
    return query.has("before")
      ? { rows: [activityRow(3, { actor: "old-timer" })], has_more: false }
      : { rows: [activityRow(5), activityRow(4)], has_more: true };
  });
  renderAt("/p/acme/demo/activity");
  await feed();
  await userEvent.click(await screen.findByRole("button", { name: "Load more" }));
  await waitFor(() => expect(screen.getByText("old-timer")).toBeInTheDocument());
  expect(queries.at(-1)).toContain("before=4");
  expect(
    within(screen.getByRole("list", { name: "Activity" })).getAllByRole("listitem"),
  ).toHaveLength(3);
  expect(screen.queryByRole("button", { name: "Load more" })).toBeNull();
});

test("a failed Load more tells the person and keeps the button", async () => {
  setup(() => ({ rows: [activityRow(5)], has_more: true }));
  renderAt("/p/acme/demo/activity");
  await feed();
  vi.stubGlobal(
    "fetch",
    vi.fn(async () => json({ code: "boom", message: "boom" }, 500)),
  );
  await userEvent.click(await screen.findByRole("button", { name: "Load more" }));
  expect(
    await screen.findByText("More activity could not be loaded. Try again."),
  ).toBeInTheDocument();
  expect(screen.getByRole("button", { name: "Load more" })).toBeEnabled();
});

test("the Live indicator is grey until the socket confirms the subscription and grey again when it drops", async () => {
  setup(() => ({ rows: [activityRow(9)], has_more: false }));
  renderAt("/p/acme/demo/activity");
  expect(await screen.findByText("Reconnecting")).toBeInTheDocument();
  await waitFor(() => expect(lastSocket()).toBeDefined());
  const socket = lastSocket() as FakeWebSocket;
  act(() => socket.open());
  expect(socket.sent).toEqual([{ subscribe: "acme/demo" }]);
  expect(screen.getByText("Reconnecting")).toBeInTheDocument();
  act(() => socket.receive({ subscribed: "acme/demo" }));
  expect(screen.getByText("Live")).toBeInTheDocument();
  act(() => socket.drop());
  expect(screen.getByText("Reconnecting")).toBeInTheDocument();
});

test("a write that arrives live shows up at the top without reloading and is highlighted for a moment", async () => {
  let rows = [activityRow(5)];
  setup(() => ({ rows, has_more: false }));
  renderAt("/p/acme/demo/activity");
  await feed();
  expect(screen.getAllByRole("listitem")).toHaveLength(1);
  const socket = connectLive("acme/demo");
  rows = [activityRow(6, { actor: "rita", target: "004/plan@2" }), activityRow(5)];
  act(() =>
    socket.receive({ project: "acme/demo", audit_id: 6, action: "document.put", result: "ok" }),
  );
  await waitFor(() => expect(screen.getAllByRole("listitem")).toHaveLength(2));
  const [top, previous] = screen.getAllByRole("listitem");
  expect(top).toHaveTextContent("rita wrote 004/plan (r2)");
  await waitFor(() => expect(top).toHaveAttribute("data-fresh", "true"));
  expect(previous).not.toHaveAttribute("data-fresh");
  await waitFor(() => expect(top).not.toHaveAttribute("data-fresh"), { timeout: 4000 });
});

test("rows loaded with Load more stay when a live refetch brings the first page again", async () => {
  let rows = [activityRow(5), activityRow(4)];
  setup((query) =>
    query.has("before")
      ? { rows: [activityRow(3, { actor: "old-timer" })], has_more: false }
      : { rows, has_more: true },
  );
  renderAt("/p/acme/demo/activity");
  await feed();
  await userEvent.click(await screen.findByRole("button", { name: "Load more" }));
  await screen.findByText("old-timer");
  const socket = connectLive("acme/demo");
  rows = [activityRow(6), activityRow(5), activityRow(4)];
  act(() => socket.receive({ resync: true }));
  await waitFor(() => expect(screen.getAllByRole("listitem")).toHaveLength(4));
  expect(screen.getByText("old-timer")).toBeInTheDocument();
});

test("the screen speaks Spanish", async () => {
  setup(() => ({
    rows: [
      activityRow(9, { action: "task.done", summary_key: "task.done", target: "004/T07@a1b2c3d" }),
      activityRow(8, {
        result: "denied",
        summary_key: "denied",
        target: "004/spec",
      }),
    ],
    has_more: false,
  }));
  renderAt("/p/acme/demo/activity", "es");
  const rows = within(await screen.findByRole("list", { name: "Actividad" })).getAllByRole(
    "listitem",
  );
  expect(rows[0]).toHaveTextContent("dana marcó T07 como hecha · commit a1b2c3d");
  expect(rows[1]).toHaveTextContent("dana intentó escribir un documento 004/spec — denegado");
  expect(screen.getByRole("button", { name: "Seguridad" })).toBeInTheDocument();
  expect(screen.getByText("Reconectando")).toBeInTheDocument();
});
