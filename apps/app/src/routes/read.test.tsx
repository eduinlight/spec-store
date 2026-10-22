import { screen, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { afterEach } from "vitest";
import { mockApi } from "~/test/api";
import { acme, me } from "~/test/fixtures";
import { renderAt } from "~/test/render";

afterEach(() => {
  vi.unstubAllGlobals();
});

const revision = (id: number, message: string) => ({
  id,
  author: "alice",
  created_at: "2026-10-22T09:00:00.000Z",
  message,
  version: null,
});

const project = {
  address: "acme/demo",
  organization: "acme",
  slug: "demo",
  name: "Demo",
  repository_url: "https://github.com/acme/demo",
  role: "reader",
  created_at: "2026-10-01T10:00:00.000Z",
};

const spec = {
  number: 1,
  slug: "first",
  status: "approved",
  branch: "001-first",
  kinds: ["spec", "tasks"],
};

function baseApi(extra: Record<string, Parameters<typeof mockApi>[0][string]> = {}) {
  return mockApi({
    "GET /api/v1/me": () => me,
    "GET /api/v1/orgs": () => ({ organizations: [acme] }),
    "GET /api/v1/projects/{org}/{project}": () => project,
    "GET /api/v1/projects/{org}/{project}/status": () => ({
      constitution_version: "1.2",
      specs: [spec],
      active: {
        ...spec,
        tasks_done: 1,
        tasks_total: 2,
        current_phase: "Phase 1",
        next_task: null,
        markers: 1,
      },
    }),
    "GET /api/v1/projects/{org}/{project}/members": () => ({
      members: [
        { handle: "alice", role: "owner" },
        { handle: "rita", role: "reader" },
      ],
    }),
    "GET /api/v1/projects/{org}/{project}/specs/{number}": () => spec,
    "GET /api/v1/projects/{org}/{project}/specs/{number}/documents/{kind}": (_r, p) =>
      p.kind === "spec"
        ? { body: "# Spec heading\n\nSome <b>text</b>\n", revision: revision(12, "second") }
        : p.kind === "tasks"
          ? { body: "# Tasks\n", revision: revision(13, "tasks") }
          : new Response(JSON.stringify({ code: "not_found", message: "missing" }), {
              status: 404,
            }),
    "GET /api/v1/projects/{org}/{project}/specs/{number}/tasks": () => ({
      tasks: [
        {
          id: "T01",
          phase: "Phase 1",
          parallel: false,
          frs: ["FR-1"],
          description: "First task",
          done_when: "tests green",
          done: true,
          commit: "abc1234",
          line: 3,
        },
        {
          id: "T02",
          phase: "Phase 1",
          parallel: true,
          frs: ["FR-2"],
          description: "Second task",
          done_when: "green",
          done: false,
          commit: null,
          line: 4,
        },
      ],
      malformed: [],
    }),
    "GET /api/v1/projects/{org}/{project}/specs/{number}/markers": () => ({
      markers: [
        {
          document_kind: "spec",
          kind: "clarification",
          heading: "Open questions",
          text: "who?",
          line: 9,
        },
      ],
    }),
    ...extra,
  });
}

test("the project page shows the constitution, specs, progress and members", async () => {
  baseApi();
  renderAt("/p/acme/demo");
  expect(await screen.findByRole("heading", { name: "Demo" })).toBeInTheDocument();
  expect(screen.getByRole("link", { name: "Version 1.2" })).toHaveAttribute(
    "href",
    "/p/acme/demo/constitution",
  );
  for (const link of screen.getAllByRole("link", { name: /001-first/ })) {
    expect(link).toHaveAttribute("href", "/p/acme/demo/specs/1");
  }
  expect(screen.getByText("1 of 2 tasks done")).toBeInTheDocument();
  expect(screen.getByRole("link", { name: "https://github.com/acme/demo" })).toBeInTheDocument();
  const members = screen.getByRole("list", { name: "Members" });
  expect(
    within(members)
      .getAllByRole("listitem")
      .map((li) => li.textContent),
  ).toEqual([expect.stringContaining("alice"), expect.stringContaining("rita")]);
});

test("the spec page renders each document, the tasks table with commit links and the markers", async () => {
  baseApi();
  renderAt("/p/acme/demo/specs/1");
  expect(await screen.findByRole("heading", { name: "Spec heading" })).toBeInTheDocument();
  expect(screen.getByText("Some <b>text</b>")).toBeInTheDocument();
  expect(screen.getAllByText("Not written yet").length).toBeGreaterThan(0);
  const table = screen.getByRole("table", { name: "Tasks" });
  expect(within(table).getByRole("link", { name: "abc1234" })).toHaveAttribute(
    "href",
    "https://github.com/acme/demo/commit/abc1234",
  );
  expect(within(table).getByText("Second task")).toBeInTheDocument();
  expect(screen.getByText("Markers (1)")).toBeInTheDocument();
  expect(screen.getByText("who?")).toBeInTheDocument();
});

test("a document can be shown raw, byte for byte", async () => {
  baseApi();
  renderAt("/p/acme/demo/specs/1/spec");
  expect(await screen.findByRole("heading", { name: "Spec heading" })).toBeInTheDocument();
  await userEvent.click(screen.getByRole("button", { name: "Raw" }));
  expect(screen.getByTestId("raw").textContent).toBe("# Spec heading\n\nSome <b>text</b>\n");
});

test("history lists revisions, older revisions say they are not current and diffs mark changes", async () => {
  baseApi({
    "GET /api/v1/projects/{org}/{project}/revisions": () => ({
      revisions: [revision(12, "second"), revision(11, "first")],
    }),
    "GET /api/v1/projects/{org}/{project}/revisions/{id}": (_r, p) => ({
      body: p.id === "11" ? "# Old\n" : "# New\n",
      kind: "spec",
      number: 1,
      revision: revision(Number(p.id), p.id === "11" ? "first" : "second"),
      current: p.id === "12",
      current_revision: 12,
    }),
    "GET /api/v1/projects/{org}/{project}/revisions/{id}/diff": () => ({
      revision: 12,
      previous_revision: 11,
      hunks: [
        {
          lines: [
            { tag: "delete", old_line: 1, new_line: null, text: "# Old" },
            { tag: "insert", old_line: null, new_line: 1, text: "# New" },
          ],
        },
      ],
    }),
  });
  const { router } = renderAt("/p/acme/demo/history?spec=1&kind=spec");
  const rows = await screen.findAllByRole("row");
  expect(rows.length).toBe(3);
  expect(within(rows[1] as HTMLElement).getByText("Current")).toBeInTheDocument();
  await router.navigate("/p/acme/demo/revisions/11");
  expect(await screen.findByText("This is not the current revision.")).toBeInTheDocument();
  expect(screen.getByRole("link", { name: "Show the current revision" })).toHaveAttribute(
    "href",
    "/p/acme/demo/revisions/12",
  );
  await router.navigate("/p/acme/demo/revisions/12/diff");
  expect(await screen.findByText("- # Old")).toBeInTheDocument();
  expect(screen.getByText("+ # New")).toBeInTheDocument();
});

test("the organization audit trail pages through older entries", async () => {
  const entry = (id: number) => ({
    id,
    actor: "alice",
    organization: "acme",
    project: "acme/demo",
    action: "spec.create",
    target: `00${id}-x`,
    created_at: "2026-10-22T09:00:00.000Z",
  });
  const api = baseApi({
    "GET /api/v1/orgs/{org}": () => ({ ...acme, created_at: "2026-10-01T10:00:00.000Z" }),
    "GET /api/v1/orgs/{org}/audit": (request) =>
      new URL(request.url).searchParams.get("before")
        ? { entries: [entry(1)], has_more: false }
        : { entries: [entry(3), entry(2)], has_more: true },
  });
  renderAt("/o/acme/audit");
  expect(await screen.findByText("003-x")).toBeInTheDocument();
  await userEvent.click(screen.getByRole("link", { name: "Older" }));
  expect(await screen.findByText("001-x")).toBeInTheDocument();
  expect(api.calls.some((c) => c.path.includes("before=2"))).toBe(true);
});
