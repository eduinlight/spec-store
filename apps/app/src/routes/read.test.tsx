import { screen, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { afterEach } from "vitest";
import { mockApi } from "~/test/api";
import { acme, me } from "~/test/fixtures";
import { renderAt } from "~/test/render";

afterEach(() => {
  vi.unstubAllGlobals();
});

const revision = (id: number, message: string, client: string | null = null) => ({
  id,
  author: "alice",
  created_at: "2026-10-22T09:00:00.000Z",
  message,
  version: null,
  client,
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

test("history lists revisions, older revisions say they are not current and diffs mark changes", async () => {
  baseApi({
    "GET /api/v1/projects/{org}/{project}/revisions": () => ({
      revisions: [revision(12, "second", "Cursor"), revision(11, "first")],
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
  expect(screen.getByRole("columnheader", { name: "Client" })).toBeInTheDocument();
  expect(within(rows[1] as HTMLElement).getByText("Cursor")).toBeInTheDocument();
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
    client: "Cursor",
    origin: "203.0.113.7",
    result: "ok",
    detail: null,
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

test("the audit trail shows the client, the origin and the result of every entry", async () => {
  const entry = (id: number, result: string, detail: string | null) => ({
    id,
    actor: "alice",
    organization: "acme",
    project: "acme/demo",
    action: "document.put",
    target: `001/spec-${id}`,
    created_at: "2026-10-22T09:00:00.000Z",
    client: id === 3 ? null : "Claude Code",
    origin: id === 3 ? null : "198.51.100.4",
    result,
    detail,
  });
  baseApi({
    "GET /api/v1/orgs/{org}": () => ({ ...acme, created_at: "2026-10-01T10:00:00.000Z" }),
    "GET /api/v1/orgs/{org}/audit": () => ({
      entries: [
        entry(3, "ok", null),
        entry(2, "conflict", "expected r12, current r13"),
        entry(1, "denied", null),
      ],
      has_more: false,
    }),
  });
  renderAt("/o/acme/audit");
  expect(await screen.findByText("001/spec-3")).toBeInTheDocument();
  expect(screen.getByRole("columnheader", { name: "Via" })).toBeInTheDocument();
  expect(screen.getByRole("columnheader", { name: "Origin" })).toBeInTheDocument();
  expect(screen.getByRole("columnheader", { name: "Result" })).toBeInTheDocument();
  const rows = screen.getAllByRole("row");
  expect(within(rows[1] as HTMLElement).getByText("OK")).toBeInTheDocument();
  expect(within(rows[2] as HTMLElement).getByText("Claude Code")).toBeInTheDocument();
  expect(within(rows[2] as HTMLElement).getByText("198.51.100.4")).toBeInTheDocument();
  expect(within(rows[2] as HTMLElement).getByText("Conflict")).toBeInTheDocument();
  expect(within(rows[2] as HTMLElement).getByText("expected r12, current r13")).toBeInTheDocument();
  expect(within(rows[3] as HTMLElement).getByText("Denied")).toBeInTheDocument();
});
