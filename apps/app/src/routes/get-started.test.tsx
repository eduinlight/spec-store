import { screen, waitFor, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { afterEach } from "vitest";
import { mockApi } from "~/test/api";
import { acme, me, summary } from "~/test/fixtures";
import { renderAt } from "~/test/render";

afterEach(() => {
  vi.unstubAllGlobals();
  vi.unstubAllEnvs();
});

const project = {
  address: "acme/demo",
  organization: "acme",
  slug: "demo",
  name: "Demo",
  repository_url: null,
  role: "owner",
  archived: false,
  created_at: "2026-10-24T10:00:00.000Z",
};

const spec = { number: 1, slug: "first", status: "draft", branch: "001-first", kinds: ["spec"] };

function setup({
  role = "owner",
  archived = false,
  specs = [] as (typeof spec)[],
  activity = null as { actor: string; client: string | null; at: string } | null,
} = {}) {
  return mockApi({
    "GET /api/v1/me": () => ({ ...me, mcp_url: "https://mcp.example.test/mcp" }),
    "GET /api/v1/orgs": () => ({ organizations: [acme] }),
    "GET /api/v1/orgs/{org}/projects": () => ({
      projects: [summary({ role, archived, last_activity: activity })],
    }),
    "GET /api/v1/projects/{org}/{project}": () => ({ ...project, role, archived }),
    "GET /api/v1/projects/{org}/{project}/status": () => ({
      constitution_version: null,
      specs,
      active: null,
    }),
    "GET /api/v1/projects/{org}/{project}/members": () => ({
      members: [{ handle: "alice", role: "owner" }],
    }),
    "GET /api/v1/orgs/{org}/members": () => ({ members: [] }),
    "POST /api/v1/projects/{org}/{project}/archive": () => new Response(null, { status: 204 }),
    "POST /api/v1/projects/{org}/{project}/restore": () => new Response(null, { status: 204 }),
  });
}

const steps = () => within(screen.getByRole("list", { name: "Get started" }));
const step = (name: string) => within(steps().getByRole("listitem", { name: new RegExp(name) }));

test("a new project shows the checklist with the connect command and nothing done but the first step", async () => {
  setup();
  renderAt("/p/acme/demo");
  expect(await screen.findByRole("heading", { name: "Get started" })).toBeInTheDocument();
  expect(screen.getByText("1 of 4 done")).toBeInTheDocument();
  expect(step("Create project").getByText("Done")).toBeInTheDocument();
  expect(
    step("Connect your agent").getByText(
      "claude mcp add --transport http sdd-vault https://mcp.example.test/mcp",
    ),
  ).toBeInTheDocument();
  expect(step("First agent call").queryByText("Done")).toBeNull();
  expect(step("Write spec 001").queryByText("Done")).toBeNull();
});

test("the connect command can be copied", async () => {
  setup();
  const user = userEvent.setup();
  renderAt("/p/acme/demo");
  await user.click(await screen.findByRole("button", { name: "Copy command" }));
  expect(await navigator.clipboard.readText()).toBe(
    "claude mcp add --transport http sdd-vault https://mcp.example.test/mcp",
  );
  expect(await screen.findByText("Command copied.")).toBeInTheDocument();
});

test("the first call by an MCP client completes the connect and first call steps", async () => {
  setup({ activity: { actor: "rita", client: "Claude Code", at: "2026-10-24T09:00:00.000Z" } });
  renderAt("/p/acme/demo");
  await screen.findByRole("heading", { name: "Get started" });
  expect(screen.getByText("3 of 4 done")).toBeInTheDocument();
  expect(step("Connect your agent").getByText("Done")).toBeInTheDocument();
  expect(step("First agent call").getByText("Done")).toBeInTheDocument();
});

test.each(["Web app", "sddvault CLI", null])(
  "activity from %s is not an agent call",
  async (client) => {
    setup({ activity: { actor: "alice", client, at: "2026-10-24T09:00:00.000Z" } });
    renderAt("/p/acme/demo");
    await screen.findByRole("heading", { name: "Get started" });
    expect(screen.getByText("1 of 4 done")).toBeInTheDocument();
  },
);

test("the checklist goes away once spec 001 exists and is never shown to readers", async () => {
  setup({ specs: [spec] });
  const owner = renderAt("/p/acme/demo");
  expect(await screen.findByRole("heading", { name: "Demo" })).toBeInTheDocument();
  expect(screen.queryByRole("heading", { name: "Get started" })).toBeNull();
  owner.unmount();
  vi.unstubAllGlobals();

  setup({ role: "reader" });
  renderAt("/p/acme/demo");
  expect(await screen.findByRole("heading", { name: "Demo" })).toBeInTheDocument();
  expect(screen.queryByRole("heading", { name: "Get started" })).toBeNull();
});

test("an archived project says it is read-only and has no checklist", async () => {
  setup({ archived: true });
  renderAt("/p/acme/demo");
  expect(await screen.findByRole("heading", { name: /Demo/ })).toBeInTheDocument();
  expect(screen.getByText("Archived")).toBeInTheDocument();
  expect(
    screen.getByText(
      "This project is archived and read-only. An owner can restore it from its settings.",
    ),
  ).toBeInTheDocument();
  expect(screen.queryByRole("heading", { name: "Get started" })).toBeNull();
});

test("owners archive a project after confirming", async () => {
  const api = setup();
  renderAt("/p/acme/demo/settings");
  await userEvent.click(await screen.findByRole("button", { name: "Archive project" }));
  const dialog = await screen.findByRole("dialog", { name: "Archive Demo?" });
  expect(api.calls.some((c) => c.path.endsWith("/archive"))).toBe(false);
  await userEvent.click(within(dialog).getByRole("button", { name: "Archive project" }));
  await waitFor(() =>
    expect(
      api.calls.some((c) => c.method === "POST" && c.path === "/api/v1/projects/acme/demo/archive"),
    ).toBe(true),
  );
  expect(await screen.findByText("Project archived.")).toBeInTheDocument();
});

test("owners restore an archived project after confirming", async () => {
  const api = setup({ archived: true });
  renderAt("/p/acme/demo/settings");
  expect(
    await screen.findByText(
      "This project is archived. Restoring it lets members write to it again.",
    ),
  ).toBeInTheDocument();
  await userEvent.click(screen.getByRole("button", { name: "Restore project" }));
  const dialog = await screen.findByRole("dialog", { name: "Restore Demo?" });
  await userEvent.click(within(dialog).getByRole("button", { name: "Restore project" }));
  await waitFor(() =>
    expect(
      api.calls.some((c) => c.method === "POST" && c.path === "/api/v1/projects/acme/demo/restore"),
    ).toBe(true),
  );
  expect(await screen.findByText("Project restored.")).toBeInTheDocument();
});

test("the checklist speaks Spanish", async () => {
  setup();
  renderAt("/p/acme/demo", "es");
  expect(await screen.findByRole("heading", { name: "Primeros pasos" })).toBeInTheDocument();
  expect(screen.getByText("1 de 4 hechos")).toBeInTheDocument();
  expect(screen.getByRole("button", { name: "Copiar comando" })).toBeInTheDocument();
});
