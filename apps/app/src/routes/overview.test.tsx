import { act, screen, waitFor, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { afterEach } from "vitest";
import { mockApi } from "~/test/api";
import { connectLive, FakeWebSocket } from "~/test/fake-socket";
import {
  acme,
  activeSpec,
  activityRow,
  me,
  overviewData,
  projectDetail,
  summary,
} from "~/test/fixtures";
import { renderAt } from "~/test/render";

afterEach(() => {
  vi.unstubAllGlobals();
  vi.useRealTimers();
  FakeWebSocket.instances = [];
});

interface Options {
  overview?: (days: string | null) => unknown;
  active?: unknown;
  rows?: unknown[];
  role?: string;
  specs?: { number: number }[];
}

function setup({
  overview = () => overviewData(),
  active = { active: activeSpec(), drafts: [] },
  rows = [],
  role = "owner",
  specs = [{ number: 1 }],
}: Options = {}) {
  return mockApi({
    "GET /api/v1/me": () => ({ ...me, mcp_url: "https://mcp.example.test/mcp" }),
    "GET /api/v1/orgs": () => ({ organizations: [acme] }),
    "GET /api/v1/orgs/{org}/projects": () => ({ projects: [summary({ role })] }),
    "GET /api/v1/projects/{org}/{project}": () => projectDetail({ role }),
    "GET /api/v1/projects/{org}/{project}/status": () => ({
      constitution_version: "1.0",
      specs,
      active: null,
    }),
    "GET /api/v1/projects/{org}/{project}/overview": (request) =>
      overview(new URL(request.url).searchParams.get("days")),
    "GET /api/v1/projects/{org}/{project}/active": () => active,
    "GET /api/v1/projects/{org}/{project}/activity": () => ({ rows, has_more: false }),
  });
}

test("the overview shows the five KPIs with the change against the previous period", async () => {
  setup();
  renderAt("/p/acme/demo");
  const kpis = await screen.findByRole("region", { name: "Overview" });
  const text = kpis.textContent ?? "";
  expect(within(kpis).getByText("Active specs")).toBeInTheDocument();
  expect(text).toContain("Tasks completed");
  expect(within(kpis).getByText("9")).toBeInTheDocument();
  expect(within(kpis).getByText("+3 vs previous 7 days")).toBeInTheDocument();
  expect(within(kpis).getByText("75%")).toBeInTheDocument();
  expect(within(kpis).getByText("3 of 4 requirements · spec 004")).toBeInTheDocument();
  expect(within(kpis).getByText("Open markers")).toBeInTheDocument();
  expect(within(kpis).getByText("Agents active")).toBeInTheDocument();
});

test("a drop in tasks completed is shown as such and no coverage shows a dash", async () => {
  setup({
    overview: () =>
      overviewData({ tasks_completed: 2, tasks_completed_previous: 5, coverage: null }),
  });
  renderAt("/p/acme/demo");
  expect(await screen.findByText("-3 vs previous 7 days")).toBeInTheDocument();
  expect(screen.getByText("No active spec with requirements")).toBeInTheDocument();
});

test("each KPI explains itself in a tooltip", async () => {
  setup();
  renderAt("/p/acme/demo");
  const tip = await screen.findByRole("button", { name: "Requirement coverage" });
  await userEvent.hover(tip);
  expect(await screen.findByRole("tooltip")).toHaveTextContent(
    "Share of the requirements of the active spec that at least one task covers.",
  );
});

test("the chart stacks the tasks completed per day by client", async () => {
  setup();
  renderAt("/p/acme/demo");
  const chart = await screen.findByRole("img", { name: /Tasks completed per day by client/ });
  expect(chart).toHaveAccessibleName(/Claude Code total 7; Cursor total 2/);
  const legend = screen.getByRole("list", { name: "Legend" });
  expect(within(legend).getByText("Claude Code")).toBeInTheDocument();
  expect(within(legend).getByText("Cursor")).toBeInTheDocument();
});

test("a period without completed tasks says so instead of drawing an empty chart", async () => {
  setup({
    overview: () => overviewData({ tasks_completed: 0, completed_per_day: [] }),
  });
  renderAt("/p/acme/demo");
  expect(await screen.findByText("No task was completed in this period.")).toBeInTheDocument();
  expect(screen.queryByRole("img", { name: /Tasks completed per day/ })).toBeNull();
});

test("choosing a period asks for it and keeps it in the address", async () => {
  const periods: (string | null)[] = [];
  setup({
    overview: (days) => {
      periods.push(days);
      return overviewData({ days: Number(days ?? 7) });
    },
  });
  const { router } = renderAt("/p/acme/demo");
  const group = await screen.findByRole("group", { name: "Period" });
  expect(within(group).getByRole("button", { name: "7 days" })).toHaveAttribute(
    "aria-pressed",
    "true",
  );
  await userEvent.click(within(group).getByRole("button", { name: "30 days" }));
  await waitFor(() => expect(periods).toContain("30"));
  expect(router.state.location.search).toBe("?days=30");
  expect(await screen.findByRole("button", { name: "30 days" })).toHaveAttribute(
    "aria-pressed",
    "true",
  );
});

test("an unknown period falls back to seven days", async () => {
  const periods: (string | null)[] = [];
  setup({
    overview: (days) => {
      periods.push(days);
      return overviewData();
    },
  });
  renderAt("/p/acme/demo?days=9");
  await screen.findByRole("group", { name: "Period" });
  expect(periods).toEqual(["7"]);
});

test("attention items open the screen where the person can act", async () => {
  setup({
    overview: () =>
      overviewData({
        attention: [
          { kind: "approval_waiting", spec_number: 5, spec_slug: "x", count: 1, detail: [] },
          {
            kind: "open_questions",
            spec_number: 4,
            spec_slug: "refunds",
            count: 2,
            detail: ["Which currencies?", "Who approves?"],
          },
          {
            kind: "requirement_without_work",
            spec_number: 4,
            spec_slug: "refunds",
            count: 1,
            detail: ["FR-3"],
          },
        ],
      }),
  });
  const { router } = renderAt("/p/acme/demo");
  const section = (await screen.findByRole("heading", { name: "Needs your attention" })).closest(
    "section",
  ) as HTMLElement;
  expect(within(section).getByText("3")).toBeInTheDocument();
  expect(within(section).getByText("Spec 005 is waiting for approval")).toBeInTheDocument();
  expect(within(section).getByText("Spec 004 has 2 open questions")).toBeInTheDocument();
  expect(within(section).getByText("Which currencies? · Who approves?")).toBeInTheDocument();
  expect(within(section).getByText("Spec 004: 1 requirement without work")).toBeInTheDocument();

  await userEvent.click(within(section).getByRole("button", { name: "Answer 004" }));
  await waitFor(() => expect(router.state.location.pathname).toBe("/p/acme/demo/specs/4/markers"));
});

test.each([
  ["Review 005", "/p/acme/demo/specs/5"],
  ["Trace 004", "/p/acme/demo/specs/4/traceability"],
])("%s navigates to %s", async (name, path) => {
  setup({
    overview: () =>
      overviewData({
        attention: [
          { kind: "approval_waiting", spec_number: 5, spec_slug: "x", count: 1, detail: [] },
          {
            kind: "requirement_without_work",
            spec_number: 4,
            spec_slug: "refunds",
            count: 1,
            detail: [],
          },
        ],
      }),
  });
  const { router } = renderAt("/p/acme/demo");
  await userEvent.click(await screen.findByRole("button", { name }));
  await waitFor(() => expect(router.state.location.pathname).toBe(path));
});

test("nothing to attend to says so", async () => {
  setup();
  renderAt("/p/acme/demo");
  expect(await screen.findByText("Nothing needs your attention.")).toBeInTheDocument();
});

test("active specs and recent activity summarize the project with links to their screens", async () => {
  setup({
    active: {
      active: activeSpec(),
      drafts: [{ number: 5, slug: "partial", title: "Partial refunds", summary: "", markers: [] }],
    },
    rows: [
      activityRow(9, { action: "task.done", summary_key: "task.done", target: "004/T07@a1b2c3d" }),
      activityRow(8),
    ],
  });
  const { router } = renderAt("/p/acme/demo");
  const link = await screen.findByRole("link", { name: /Refund flow/ });
  expect(link).toHaveAttribute("href", "/p/acme/demo/specs/4");
  expect(within(link).getByRole("progressbar")).toHaveAttribute("aria-valuenow", "12");
  expect(within(link).getByText("12 / 20")).toBeInTheDocument();
  expect(screen.getByRole("link", { name: /Partial refunds/ })).toHaveTextContent(
    "In specification · 005-partial",
  );
  const recent = screen.getByRole("list", { name: "Recent activity" });
  expect(within(recent).getAllByRole("listitem")).toHaveLength(2);
  expect(within(recent).getByText(/marked T07 done · commit a1b2c3d/)).toBeInTheDocument();

  await userEvent.click(screen.getByRole("button", { name: "View all: Active specs" }));
  await waitFor(() => expect(router.state.location.pathname).toBe("/p/acme/demo/active"));
});

test("recent activity links to the activity screen", async () => {
  setup({ rows: [activityRow(9)] });
  const { router } = renderAt("/p/acme/demo");
  await userEvent.click(await screen.findByRole("button", { name: "View all: Recent activity" }));
  await waitFor(() => expect(router.state.location.pathname).toBe("/p/acme/demo/activity"));
});

test("without active specs or activity the cards say so", async () => {
  setup({ active: { active: null, drafts: [] } });
  renderAt("/p/acme/demo");
  expect(await screen.findByText("No active specs.")).toBeInTheDocument();
  expect(screen.getByText("Nothing has happened in this project yet.")).toBeInTheDocument();
});

test("the get started checklist stays on top for an owner who has not written spec 001", async () => {
  setup({ specs: [] });
  renderAt("/p/acme/demo");
  expect(await screen.findByRole("heading", { name: "Get started" })).toBeInTheDocument();
  expect(screen.getByRole("region", { name: "Overview" })).toBeInTheDocument();
});

test("the overview refetches once when writes arrive in a burst", async () => {
  let loads = 0;
  setup({
    overview: () => {
      loads += 1;
      return overviewData();
    },
  });
  renderAt("/p/acme/demo");
  await screen.findByRole("region", { name: "Overview" });
  expect(loads).toBe(1);
  await waitFor(() => expect(FakeWebSocket.instances.length).toBeGreaterThan(0));
  const socket = connectLive("acme/demo");
  expect(socket.sent).toEqual([{ subscribe: "acme/demo" }]);
  act(() => {
    for (const id of [1, 2, 3]) {
      socket.receive({ project: "acme/demo", audit_id: id, action: "task.done", result: "ok" });
    }
  });
  await waitFor(() => expect(loads).toBe(2));
  await new Promise((resolve) => setTimeout(resolve, 600));
  expect(loads).toBe(2);
});

test("a resync from the server refetches the overview", async () => {
  let loads = 0;
  setup({
    overview: () => {
      loads += 1;
      return overviewData();
    },
  });
  renderAt("/p/acme/demo");
  await screen.findByRole("region", { name: "Overview" });
  const socket = connectLive("acme/demo");
  act(() => socket.receive({ resync: true }));
  await waitFor(() => expect(loads).toBe(2));
});

test("the overview speaks Spanish", async () => {
  setup();
  renderAt("/p/acme/demo", "es");
  expect(await screen.findByRole("region", { name: "Resumen" })).toBeInTheDocument();
  expect(screen.getByText("Tareas completadas", { selector: "span" })).toBeInTheDocument();
  expect(screen.getByText("+3 frente a los 7 días anteriores")).toBeInTheDocument();
  expect(screen.getByRole("heading", { name: "Requiere tu atención" })).toBeInTheDocument();
  expect(screen.getByRole("button", { name: "7 días" })).toBeInTheDocument();
});
