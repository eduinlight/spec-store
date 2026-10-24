import { screen, waitFor, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { afterEach } from "vitest";
import { mockApi } from "~/test/api";
import { acme, me, projectDetail, specCard } from "~/test/fixtures";
import { renderAt } from "~/test/render";

afterEach(() => {
  vi.unstubAllGlobals();
});

const SPEC_BODY = `# Spec 001

## Summary

Intro text.

## Functional requirements

- **FR-1** (US-1): THE SYSTEM SHALL work.
- **FR-2**: THE SYSTEM SHALL fail loudly.

## Success criteria

Done.
`;

const revision = (id: number, message: string, client: string | null = null) => ({
  id,
  author: "priya",
  created_at: "2026-09-02T09:00:00.000Z",
  message,
  version: null,
  client,
});

const attribution = (author: string, client: string | null = "Cursor") => ({
  author,
  client,
  at: "2026-09-03T09:00:00.000Z",
  revision: 12,
});

const task = (id: string, overrides: Record<string, unknown> = {}) => ({
  id,
  phase: "Phase 1",
  parallel: false,
  frs: ["FR-1"],
  description: `Task ${id}`,
  done_when: "tests green",
  done: false,
  commit: null,
  line: 3,
  completed_by: null,
  ...overrides,
});

const NOT_FOUND = () =>
  new Response(JSON.stringify({ code: "not_found", message: "missing" }), { status: 404 });

function specApi(
  options: {
    project?: Record<string, unknown>;
    spec?: Record<string, unknown>;
    card?: Record<string, unknown>;
    extra?: Record<string, Parameters<typeof mockApi>[0][string]>;
  } = {},
) {
  return mockApi({
    "GET /api/v1/me": () => me,
    "GET /api/v1/orgs": () => ({ organizations: [acme] }),
    "GET /api/v1/orgs/{org}/projects": () => ({ projects: [] }),
    "GET /api/v1/projects/{org}/{project}/status": () => ({
      constitution_version: "1.0",
      specs: [],
      active: null,
    }),
    "GET /api/v1/projects/{org}/{project}": () => projectDetail(options.project),
    "GET /api/v1/projects/{org}/{project}/specs/cards": () => ({
      specs: [
        specCard(1, {
          title: "Diesel migration",
          slug: "first",
          documents: ["spec", "tasks"],
          tasks_done: 1,
          tasks_total: 3,
          open_markers: 0,
          ...options.card,
        }),
      ],
    }),
    "GET /api/v1/projects/{org}/{project}/specs/{number}": () => ({
      number: 1,
      slug: "first",
      status: "approved",
      branch: "001-first",
      kinds: ["spec", "tasks"],
      ...options.spec,
    }),
    "GET /api/v1/projects/{org}/{project}/specs/{number}/documents/{kind}": (_r, p) =>
      p.kind === "spec"
        ? { body: SPEC_BODY, revision: revision(12, "second", "Cursor") }
        : NOT_FOUND(),
    "GET /api/v1/projects/{org}/{project}/specs/{number}/readiness": () => ({
      ready: false,
      items: [
        { key: "open_markers_resolved", ok: true, count: 0, total: null },
        { key: "requirements_covered", ok: false, count: 1, total: 2 },
        { key: "plan_written", ok: false, count: null, total: null },
        { key: "tasks_written", ok: true, count: 3, total: null },
        { key: "constitution_exists", ok: true, count: null, total: null },
      ],
    }),
    "GET /api/v1/projects/{org}/{project}/specs/{number}/traceability": () => ({
      requirements: [
        {
          id: "FR-1",
          text: "THE SYSTEM SHALL work.",
          status: "in_progress",
          tasks: [
            { id: "T01", description: "Build it", done: true, commit: "abc1234" },
            { id: "T02", description: "Test it", done: false, commit: null },
          ],
        },
        { id: "FR-2", text: "THE SYSTEM SHALL fail loudly.", status: "no_tasks", tasks: [] },
      ],
      tasks_without_requirements: [{ id: "T00", description: "Setup", done: true, commit: null }],
    }),
    "GET /api/v1/projects/{org}/{project}/specs/{number}/tasks": () => ({
      tasks: [
        task("T01", {
          done: true,
          commit: "abc1234",
          completed_by: attribution("alice"),
        }),
        task("T02", { parallel: true, frs: ["FR-1", "FR-2"] }),
        task("T03", {
          phase: "Phase 2",
          done: true,
          commit: "def5678",
          frs: [],
          completed_by: attribution("rita", "Claude Code"),
        }),
      ],
      malformed: [],
    }),
    "GET /api/v1/projects/{org}/{project}/specs/{number}/markers": () => ({
      markers: [],
      resolved: [
        {
          document_kind: "spec",
          kind: "clarification",
          heading: "Open questions",
          text: "Who approves refunds?",
          resolved_by: attribution("rita", "Claude Code"),
        },
        {
          document_kind: "plan",
          kind: "research",
          heading: "Risks",
          text: "Which driver?",
          resolved_by: { ...attribution("alice"), at: "2026-09-05T09:00:00.000Z" },
        },
      ],
    }),
    "GET /api/v1/projects/{org}/{project}/revisions": () => ({
      revisions: [revision(12, "second", "Cursor"), revision(11, "first")],
    }),
    "GET /api/v1/projects/{org}/{project}/revisions/{id}/diff": (_r, p) => ({
      revision: Number(p.id),
      previous_revision: p.id === "12" ? 11 : null,
      hunks: [
        {
          lines: [
            { tag: "delete", old_line: 1, new_line: null, text: "# Old" },
            { tag: "insert", old_line: null, new_line: 1, text: "# New" },
            { tag: "insert", old_line: null, new_line: 2, text: "Extra" },
          ],
        },
      ],
    }),
    ...options.extra,
  });
}

test("the header shows number, title, status, branch, lifecycle and tabs with counts", async () => {
  specApi();
  renderAt("/p/acme/demo/specs/1");
  expect(await screen.findByRole("heading", { level: 1, name: "Diesel migration" })).toBeVisible();
  expect(screen.getAllByText("001").length).toBeGreaterThan(0);
  expect(screen.getAllByText("Approved").length).toBeGreaterThan(0);
  expect(screen.getByText(/001-first/)).toBeInTheDocument();
  const crumbs = screen.getByRole("navigation", { name: "Breadcrumb" });
  expect(within(crumbs).getByRole("button", { name: "Demo" })).toBeInTheDocument();
  expect(within(crumbs).getByRole("button", { name: "Specs" })).toBeInTheDocument();
  const current = document.querySelector('li[aria-current="step"]');
  expect(current?.textContent).toContain("Approved");
  expect(screen.getByRole("tab", { name: "Spec" })).toHaveAttribute("aria-selected", "true");
  expect(screen.getByRole("tab", { name: "Tasks · 1/3" })).toBeInTheDocument();
  expect(screen.getByRole("tab", { name: "Markers · 0" })).toBeInTheDocument();
  expect(screen.getByRole("tab", { name: "Validation" })).toHaveAttribute("aria-disabled", "true");
  expect(screen.getByRole("tab", { name: "Traceability" })).not.toHaveAttribute("aria-disabled");
});

test("the spec tab shows the revision line, the rendered document, the contents and the readiness", async () => {
  specApi();
  renderAt("/p/acme/demo/specs/1");
  expect(await screen.findByRole("heading", { name: "Functional requirements" })).toBeVisible();
  expect(screen.getByText(/Revision 2 of 2 · by priya · via Cursor/)).toBeInTheDocument();
  const contents = screen.getByRole("navigation", { name: "On this page" });
  expect(within(contents).getByRole("link", { name: "Summary" })).toHaveAttribute(
    "href",
    "#summary",
  );
  expect(within(contents).getByRole("link", { name: "Functional requirements" })).toHaveAttribute(
    "href",
    "#functional-requirements",
  );
  expect(screen.getByRole("heading", { name: "Summary" })).toHaveAttribute("id", "summary");
  const readiness = screen.getByRole("region", { name: "Readiness" });
  expect(within(readiness).getByText("3 of 5")).toBeInTheDocument();
  expect(within(readiness).getByText("Every requirement has a task")).toBeInTheDocument();
  expect(within(readiness).getByText("1 of 2 covered")).toBeInTheDocument();
  expect(within(readiness).getByText("No open markers")).toBeInTheDocument();
  expect(within(readiness).getByText("3 tasks")).toBeInTheDocument();
  expect(within(readiness).getAllByText("Missing")).toHaveLength(1);
  expect(screen.queryByText(/Approvals/i)).not.toBeInTheDocument();
});

test("a requirement shows the tasks that reference it in a tooltip", async () => {
  specApi();
  renderAt("/p/acme/demo/specs/1");
  const fr1 = await screen.findByText("FR-1");
  await userEvent.hover(fr1);
  expect(await screen.findByText("Referenced by T01, T02")).toBeInTheDocument();
  await userEvent.unhover(fr1);
  await userEvent.hover(screen.getByText("FR-2"));
  expect(await screen.findByText("No task references this requirement")).toBeInTheDocument();
});

test("the document can be shown raw, byte for byte, and compare opens the history", async () => {
  specApi();
  const { router } = renderAt("/p/acme/demo/specs/1");
  await screen.findByRole("heading", { name: "Summary" });
  await userEvent.click(screen.getByRole("tab", { name: "Raw" }));
  expect(screen.getByTestId("raw").textContent).toBe(SPEC_BODY);
  await userEvent.click(screen.getByRole("tab", { name: "Rendered" }));
  expect(screen.queryByTestId("raw")).not.toBeInTheDocument();
  await userEvent.click(screen.getByRole("button", { name: "Compare" }));
  await waitFor(() =>
    expect(router.state.location.pathname + router.state.location.search).toBe(
      "/p/acme/demo/specs/1/history?kind=spec",
    ),
  );
});

test("the tabs are linkable and a document that is not written says so", async () => {
  specApi();
  const { router } = renderAt("/p/acme/demo/specs/1");
  await screen.findByRole("heading", { name: "Summary" });
  await userEvent.click(screen.getByRole("tab", { name: "Plan" }));
  expect(await screen.findByText("Plan is not written yet.")).toBeInTheDocument();
  expect(router.state.location.pathname).toBe("/p/acme/demo/specs/1/plan");
  expect(screen.getByRole("tab", { name: "Plan" })).toHaveAttribute("aria-selected", "true");
  await userEvent.click(screen.getByRole("tab", { name: "Traceability" }));
  expect(await screen.findByRole("heading", { name: "Requirement coverage" })).toBeInTheDocument();
  expect(router.state.location.pathname).toBe("/p/acme/demo/specs/1/traceability");
});

test("the old document address still opens the spec and an unknown tab is not found", async () => {
  specApi();
  const first = renderAt("/p/acme/demo/specs/1/spec");
  expect(await screen.findByRole("heading", { name: "Summary" })).toBeVisible();
  expect(screen.getByRole("tab", { name: "Spec" })).toHaveAttribute("aria-selected", "true");
  first.unmount();
  renderAt("/p/acme/demo/specs/1/nonsense");
  expect(await screen.findByRole("alert")).toHaveTextContent("This page does not exist");
});

test("tasks are grouped by phase with who completed them, through which client and the commit", async () => {
  specApi();
  renderAt("/p/acme/demo/specs/1/tasks");
  expect(await screen.findByText("Task T01")).toBeInTheDocument();
  expect(screen.getByText("2 of 3 done")).toBeInTheDocument();
  const phaseOne = screen.getByRole("region", { name: "Phase 1" });
  expect(within(phaseOne).getByText("1/2")).toBeInTheDocument();
  expect(within(phaseOne).getByText("Task T02")).toBeInTheDocument();
  expect(within(phaseOne).getAllByText("Done when: tests green")).toHaveLength(2);
  expect(within(phaseOne).getByText("Cursor")).toBeInTheDocument();
  expect(within(phaseOne).getByRole("link", { name: "abc1234" })).toHaveAttribute(
    "href",
    "https://github.com/acme/demo/commit/abc1234",
  );
  expect(within(phaseOne).getByText("P")).toBeInTheDocument();
  expect(within(phaseOne).getByText("FR-2")).toBeInTheDocument();
  const phaseTwo = screen.getByRole("region", { name: "Phase 2" });
  expect(within(phaseTwo).getByText("Claude Code")).toBeInTheDocument();
  await userEvent.hover(within(phaseOne).getAllByText("FR-1")[0] as HTMLElement);
  expect(await screen.findByText("THE SYSTEM SHALL work.")).toBeInTheDocument();
});

test("the task filters narrow to pending, done, mine and parallel tasks", async () => {
  specApi();
  renderAt("/p/acme/demo/specs/1/tasks");
  await screen.findByText("Task T01");
  await userEvent.click(screen.getByRole("button", { name: "Pending" }));
  expect(screen.queryByText("Task T01")).not.toBeInTheDocument();
  expect(screen.getByText("Task T02")).toBeInTheDocument();
  await userEvent.click(screen.getByRole("button", { name: "Done" }));
  expect(screen.getByRole("button", { name: "Pending" })).toHaveAttribute("aria-pressed", "false");
  expect(screen.getByText("Task T01")).toBeInTheDocument();
  expect(screen.getByText("Task T03")).toBeInTheDocument();
  await userEvent.click(screen.getByRole("button", { name: "Mine" }));
  expect(screen.getByText("Task T01")).toBeInTheDocument();
  expect(screen.queryByText("Task T03")).not.toBeInTheDocument();
  await userEvent.click(screen.getByRole("button", { name: "Parallel" }));
  expect(screen.getByText("No tasks match these filters.")).toBeInTheDocument();
});

test("without a tasks document the tab says it is not written", async () => {
  specApi({
    extra: {
      "GET /api/v1/projects/{org}/{project}/specs/{number}/tasks": NOT_FOUND,
    },
  });
  renderAt("/p/acme/demo/specs/1/tasks");
  expect(await screen.findByText("Tasks is not written yet.")).toBeInTheDocument();
});

test("traceability lists every requirement with its tasks, coverage, commits and status", async () => {
  specApi();
  renderAt("/p/acme/demo/specs/1/traceability");
  const table = await screen.findByRole("table", { name: "Requirement coverage" });
  expect(screen.getByText("1 of 2 requirements have tasks")).toBeInTheDocument();
  const rows = within(table).getAllByRole("row");
  expect(rows).toHaveLength(3);
  const first = within(rows[1] as HTMLElement);
  expect(first.getByText("FR-1")).toBeInTheDocument();
  expect(first.getByText("THE SYSTEM SHALL work.")).toBeInTheDocument();
  expect(first.getByText("T01")).toBeInTheDocument();
  expect(first.getByText("T02")).toBeInTheDocument();
  expect(first.getByText("1/2")).toBeInTheDocument();
  expect(first.getByRole("progressbar")).toHaveAttribute("aria-valuenow", "50");
  expect(first.getByRole("link", { name: "abc1234" })).toHaveAttribute(
    "href",
    "https://github.com/acme/demo/commit/abc1234",
  );
  expect(first.getByText("In progress")).toBeInTheDocument();
  expect(within(rows[2] as HTMLElement).getByText("No tasks")).toBeInTheDocument();
  expect(screen.getByText(/T00 have no requirement/)).toBeInTheDocument();
  await userEvent.hover(first.getByText("T02"));
  expect(await screen.findByText("T02 · Test it · pending")).toBeInTheDocument();
});

test("markers show the empty state and the resolved questions with who answered", async () => {
  specApi();
  renderAt("/p/acme/demo/specs/1/markers");
  expect(await screen.findByText("No open questions. Nice.")).toBeInTheDocument();
  expect(
    screen.getByText(
      /All 2 markers in this spec were resolved\. The last one was answered by alice/,
    ),
  ).toBeInTheDocument();
  const resolved = screen.getByRole("region", { name: "Resolved" });
  expect(within(resolved).getByText("Who approves refunds?", { exact: false })).toBeInTheDocument();
  expect(
    within(resolved).getByText(/rita · Spec · Open questions · via Claude Code/),
  ).toBeInTheDocument();
  expect(within(resolved).getAllByText(/\[NEEDS/)).toHaveLength(2);
});

test("open markers are listed above the resolved ones", async () => {
  specApi({
    extra: {
      "GET /api/v1/projects/{org}/{project}/specs/{number}/markers": () => ({
        markers: [
          {
            document_kind: "spec",
            kind: "clarification",
            heading: "Scope",
            text: "Which currency?",
            line: 9,
          },
        ],
        resolved: [],
      }),
    },
  });
  renderAt("/p/acme/demo/specs/1/markers");
  const open = await screen.findByRole("region", { name: "Open" });
  expect(within(open).getByText("Which currency?", { exact: false })).toBeInTheDocument();
  expect(within(open).getByText("[NEEDS CLARIFICATION]")).toBeInTheDocument();
  expect(screen.queryByText("No open questions. Nice.")).not.toBeInTheDocument();
});

test("history lists the revisions and shows the diff of the selected one", async () => {
  specApi();
  const { router } = renderAt("/p/acme/demo/specs/1/history");
  expect(await screen.findByText("r1 → r2 · spec.md")).toBeInTheDocument();
  expect(screen.getByText("+2")).toBeInTheDocument();
  expect(screen.getByText("−1")).toBeInTheDocument();
  expect(screen.getByText("# New")).toBeInTheDocument();
  expect(screen.getByText("Extra")).toBeInTheDocument();
  const list = screen.getByRole("list", { name: "Revisions" });
  expect(within(list).getAllByRole("listitem")).toHaveLength(2);
  expect(within(list).getByText(/priya · Cursor/)).toBeInTheDocument();
  await userEvent.click(within(list).getByRole("link", { name: /r1.*first/ }));
  expect(await screen.findByText(/r1 · first revision · spec.md/)).toBeInTheDocument();
  expect(router.state.location.search).toBe("?kind=spec&revision=11");
  expect(screen.getByRole("link", { name: "View" })).toHaveAttribute(
    "href",
    "/p/acme/demo/revisions/11",
  );
});

test("an archived spec and an archived project are read-only with a banner", async () => {
  specApi({ spec: { status: "archived" } });
  const first = renderAt("/p/acme/demo/specs/1");
  expect(await screen.findByText("This spec is archived and read-only.")).toBeInTheDocument();
  first.unmount();
  vi.unstubAllGlobals();
  specApi({ project: { archived: true } });
  renderAt("/p/acme/demo/specs/1");
  expect(await screen.findByText(/This project is archived and read-only/)).toBeInTheDocument();
});

test("the spec detail speaks Spanish", async () => {
  specApi();
  renderAt("/p/acme/demo/specs/1", "es");
  expect(await screen.findByRole("tab", { name: "Tareas · 1/3" })).toBeInTheDocument();
  expect(screen.getByRole("tab", { name: "Trazabilidad" })).toBeInTheDocument();
  expect(screen.getByRole("tab", { name: "Marcadores · 0" })).toBeInTheDocument();
  expect(await screen.findByText(/Revisión 2 de 2 · por priya · vía Cursor/)).toBeInTheDocument();
  expect(screen.getByRole("region", { name: "Preparación" })).toBeInTheDocument();
  expect(screen.getByRole("navigation", { name: "En esta página" })).toBeInTheDocument();
});
