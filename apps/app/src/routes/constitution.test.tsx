import { screen, within } from "@testing-library/react";
import { afterEach } from "vitest";
import { mockApi } from "~/test/api";
import { acme, me, projectDetail } from "~/test/fixtures";
import { renderAt } from "~/test/render";

afterEach(() => {
  vi.unstubAllGlobals();
});

const BODY = `# Principles of the payments service

Non-negotiable principles.

1. **The spec rules.** No implementation without an approved \`spec\`.
2. **Tests as a gate.** Tests are written before the code.
3. **Language.** Everything is in English.

Version 3.0
`;

const revision = (id: number, version: string | null, message: string, author = "priya") => ({
  id,
  author,
  created_at: `2026-0${id}-14T12:00:00.000Z`,
  message,
  version,
  client: id === 3 ? "Cursor" : null,
});

function constitutionApi(
  options: {
    project?: Record<string, unknown>;
    constitution?: unknown;
    revisions?: ReturnType<typeof revision>[];
  } = {},
) {
  return mockApi({
    "GET /api/v1/me": () => me,
    "GET /api/v1/orgs": () => ({ organizations: [acme] }),
    "GET /api/v1/orgs/{org}/projects": () => ({ projects: [] }),
    "GET /api/v1/projects/{org}/{project}/status": () => ({
      constitution_version: "3.0",
      specs: [],
      active: null,
    }),
    "GET /api/v1/projects/{org}/{project}": () =>
      projectDetail({ name: "Payments", ...options.project }),
    "GET /api/v1/projects/{org}/{project}/constitution": () =>
      options.constitution === undefined
        ? { body: BODY, version: "3.0", revision: revision(3, "3.0", "Third", "rita") }
        : options.constitution,
    "GET /api/v1/projects/{org}/{project}/revisions": () => ({
      revisions: options.revisions ?? [
        revision(3, "3.0", "Third version", "rita"),
        revision(2, "2.0", "Second version"),
        revision(1, "1.0", "First version"),
      ],
    }),
  });
}

test("the constitution is a document with its version, dates, principles and version history", async () => {
  constitutionApi();
  renderAt("/p/acme/demo/constitution");
  expect(
    await screen.findByRole("heading", { level: 1, name: "Principles of the payments service" }),
  ).toBeInTheDocument();
  expect(screen.getByText("Payments · Constitution")).toBeInTheDocument();
  expect(screen.getAllByText("v3.0").length).toBeGreaterThan(0);
  expect(screen.getByText(/Ratified January 14, 2026/)).toBeInTheDocument();
  expect(screen.getByText(/Amended March 14, 2026 by rita/)).toBeInTheDocument();
  const principles = screen.getByRole("list", { name: "Principles" });
  const items = within(principles).getAllByRole("listitem");
  expect(items).toHaveLength(3);
  expect(within(items[0] as HTMLElement).getByText("I")).toBeInTheDocument();
  expect(
    within(items[0] as HTMLElement).getByRole("heading", { name: "The spec rules" }),
  ).toBeInTheDocument();
  expect(within(items[0] as HTMLElement).getByText("spec").tagName).toBe("CODE");
  expect(within(items[2] as HTMLElement).getByText("III")).toBeInTheDocument();
  const history = screen.getByRole("region", { name: "Version history" });
  expect(within(history).getAllByRole("listitem")).toHaveLength(3);
  expect(within(history).getByRole("link", { name: "v3.0" })).toHaveAttribute(
    "href",
    "/p/acme/demo/revisions/3",
  );
  expect(within(history).getByText(/rita · via Cursor/)).toBeInTheDocument();
  expect(within(history).getByRole("link", { name: "Full history" })).toHaveAttribute(
    "href",
    "/p/acme/demo/history?kind=constitution",
  );
});

test("owners are told amendments create a version and everyone else sees the lock note", async () => {
  constitutionApi();
  const owner = renderAt("/p/acme/demo/constitution");
  expect(
    await screen.findByText("You are an owner. Amendments create a new version."),
  ).toBeInTheDocument();
  owner.unmount();
  vi.unstubAllGlobals();
  constitutionApi({ project: { role: "reader" } });
  renderAt("/p/acme/demo/constitution");
  expect(
    await screen.findByText("Locked. Only project owners can amend the constitution."),
  ).toBeInTheDocument();
});

test("a single revision is only ratified and a document without principles is shown as markdown", async () => {
  constitutionApi({
    constitution: {
      body: "# Constitution\n\nVersion 1.0\n\n- plain list\n",
      version: "1.0",
      revision: revision(1, "1.0", "First version"),
    },
    revisions: [revision(1, "1.0", "First version")],
  });
  renderAt("/p/acme/demo/constitution");
  expect(await screen.findByText(/Ratified January 14, 2026/)).toBeInTheDocument();
  expect(screen.queryByText(/Amended/)).not.toBeInTheDocument();
  expect(screen.queryByRole("list", { name: "Principles" })).not.toBeInTheDocument();
  expect(screen.getByText("plain list")).toBeInTheDocument();
});

test("a project without a constitution says so and an archived one shows the banner", async () => {
  constitutionApi({
    project: { archived: true },
    constitution: new Response(JSON.stringify({ code: "not_found", message: "none" }), {
      status: 404,
    }),
    revisions: [],
  });
  renderAt("/p/acme/demo/constitution");
  expect(await screen.findByText("No constitution yet")).toBeInTheDocument();
  expect(screen.getByText(/archived and read-only/)).toBeInTheDocument();
});

test("the constitution page speaks Spanish", async () => {
  constitutionApi({ project: { role: "reader" } });
  renderAt("/p/acme/demo/constitution", "es");
  expect(await screen.findByText("Payments · Constitución")).toBeInTheDocument();
  expect(screen.getByText(/Ratificada 14 de enero de 2026/)).toBeInTheDocument();
  expect(screen.getByText(/Enmendada 14 de marzo de 2026 por rita/)).toBeInTheDocument();
  expect(screen.getByRole("region", { name: "Historial de versiones" })).toBeInTheDocument();
  expect(
    screen.getByText(
      "Bloqueada. Solo los propietarios del proyecto pueden enmendar la constitución.",
    ),
  ).toBeInTheDocument();
});
