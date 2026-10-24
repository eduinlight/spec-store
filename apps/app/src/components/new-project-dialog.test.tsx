import { screen, waitFor, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { afterEach } from "vitest";
import { json, mockApi } from "~/test/api";
import { acme, me, summary } from "~/test/fixtures";
import { renderAt } from "~/test/render";

afterEach(() => {
  vi.unstubAllGlobals();
});

const created = {
  address: "acme/payments-service",
  organization: "acme",
  slug: "payments-service",
  name: "Payments service",
  repository_url: null,
  role: "owner",
  archived: false,
  created_at: "2026-10-24T10:00:00.000Z",
};

const demo = summary({ address: "acme/demo", slug: "demo", name: "Demo" });

function setup(extra: Record<string, Parameters<typeof mockApi>[0][string]> = {}, role = "owner") {
  return mockApi({
    "GET /api/v1/me": () => me,
    "GET /api/v1/orgs": () => ({ organizations: [{ ...acme, role }] }),
    "GET /api/v1/orgs/{org}": () => ({ ...acme, role, created_at: "2026-10-01T10:00:00.000Z" }),
    "GET /api/v1/orgs/{org}/projects": () => ({ projects: [demo] }),
    "GET /api/v1/projects/{org}/{project}": () => created,
    "GET /api/v1/projects/{org}/{project}/status": () => ({
      constitution_version: null,
      specs: [],
      active: null,
    }),
    "GET /api/v1/projects/{org}/{project}/members": () => ({
      members: [{ handle: "alice", role: "owner" }],
    }),
    ...extra,
  });
}

const dialog = async () => within(await screen.findByRole("dialog", { name: "New project" }));

function called(api: ReturnType<typeof mockApi>, method: string, path: string) {
  return api.calls.filter((c) => c.method === method && c.path === path);
}

function folderFile(path: string, body: string) {
  const file = new File([body], path.split("/").pop() ?? path);
  Object.defineProperty(file, "webkitRelativePath", { value: `repo/${path}` });
  return file;
}

const goodTree = [
  folderFile("docs/constitution.md", "# Constitution\n\nVersion 1.0\n"),
  folderFile("specs/001-first/spec.md", "Status: approved\n"),
  folderFile("specs/001-first/tasks.md", "- [ ] T01 Do it\n"),
  folderFile("specs/001-first/notes.md", "notes"),
  folderFile("README.md", "readme"),
];

test("New project opens as a dialog over the project cards", async () => {
  setup();
  const { router } = renderAt("/o/acme");
  await userEvent.click(await screen.findByRole("button", { name: "Create project" }));
  expect(router.state.location.pathname).toBe("/o/acme/projects/new");
  const form = await dialog();
  expect(form.getByLabelText("Name")).toBeInTheDocument();
  expect(screen.getByRole("heading", { name: "Projects" })).toBeInTheDocument();
  await userEvent.click(form.getByRole("button", { name: "Cancel" }));
  await waitFor(() => expect(router.state.location.pathname).toBe("/o/acme"));
  expect(screen.queryByRole("dialog")).toBeNull();
});

test("Import SDD tree opens the dialog with that start option selected", async () => {
  setup();
  renderAt("/o/acme");
  await userEvent.click(await screen.findByRole("button", { name: "Import SDD tree" }));
  const form = await dialog();
  expect(form.getByRole("radio", { name: /Import SDD tree/ })).toBeChecked();
  expect(
    form.getByText("A dry run lists every problem before anything is imported."),
  ).toBeVisible();
});

test("a member who opens the address sees no dialog", async () => {
  setup({}, "member");
  renderAt("/o/acme/projects/new");
  await screen.findByRole("heading", { name: "Projects" });
  expect(screen.queryByRole("dialog")).toBeNull();
});

test("the slug follows the name until it is edited and the preview shows how agents address it", async () => {
  setup();
  renderAt("/o/acme/projects/new");
  const form = await dialog();
  await userEvent.type(form.getByLabelText("Name"), "Payments service");
  expect(form.getByLabelText("Slug")).toHaveValue("payments-service");
  expect(form.getByText("acme/payments-service")).toBeInTheDocument();
  expect(form.getByText(/Agents address the project by slug:/)).toBeInTheDocument();
  await userEvent.clear(form.getByLabelText("Slug"));
  await userEvent.type(form.getByLabelText("Slug"), "pay");
  await userEvent.type(form.getByLabelText("Name"), " v2");
  expect(form.getByLabelText("Slug")).toHaveValue("pay");
  expect(form.getByText("acme/pay")).toBeInTheDocument();
  await userEvent.clear(form.getByLabelText("Slug"));
  await userEvent.type(form.getByLabelText("Slug"), "Bad Slug");
  expect(form.getByText(/Use one to four lowercase words/)).toBeInTheDocument();
  expect(form.getByRole("button", { name: "Create project" })).toBeDisabled();
});

test("the organization constitution option is disabled with a note", async () => {
  setup();
  renderAt("/o/acme/projects/new");
  const form = await dialog();
  expect(form.getByRole("switch", { name: "Inherit Acme Labs constitution" })).toBeDisabled();
  expect(
    form.getByText("Organization constitutions arrive in a later release."),
  ).toBeInTheDocument();
  expect(
    form.getByText("You become the project owner. Add members after creating it."),
  ).toBeInTheDocument();
  expect(form.getByText("Used to link task commits.")).toBeInTheDocument();
});

test("an empty project is created with its repository and opens on its overview", async () => {
  const api = setup({
    "POST /api/v1/orgs/{org}/projects": () => json(created, 201),
    "PUT /api/v1/projects/{org}/{project}/repository-url": () =>
      new Response(null, { status: 204 }),
  });
  const { router } = renderAt("/o/acme/projects/new");
  const form = await dialog();
  await userEvent.type(form.getByLabelText("Name"), "Payments service");
  await userEvent.type(form.getByLabelText(/Repository/), "https://github.com/acme/payments");
  await userEvent.click(form.getByRole("button", { name: "Create project" }));
  await waitFor(() => expect(router.state.location.pathname).toBe("/p/acme/payments-service"));
  expect(called(api, "POST", "/api/v1/orgs/acme/projects")[0]?.body).toEqual({
    slug: "payments-service",
    name: "Payments service",
  });
  expect(
    called(api, "PUT", "/api/v1/projects/acme/payments-service/repository-url")[0]?.body,
  ).toEqual({ url: "https://github.com/acme/payments" });
  expect(await screen.findByText("Project created.")).toBeInTheDocument();
});

test("a taken slug shows the error and keeps the inputs", async () => {
  setup({
    "POST /api/v1/orgs/{org}/projects": () =>
      json({ code: "conflict", message: "project acme/demo already exists" }, 409),
  });
  const { router } = renderAt("/o/acme/projects/new");
  const form = await dialog();
  await userEvent.type(form.getByLabelText("Name"), "Demo");
  await userEvent.click(form.getByRole("button", { name: "Create project" }));
  expect(await form.findByRole("alert")).toHaveTextContent("project acme/demo already exists");
  expect(form.getByLabelText("Name")).toHaveValue("Demo");
  expect(form.getByLabelText("Slug")).toHaveValue("demo");
  expect(router.state.location.pathname).toBe("/o/acme/projects/new");
});

test("copying a project sends the chosen source", async () => {
  const api = setup({
    "POST /api/v1/projects/{org}/{project}/copy": () => json(created, 201),
  });
  const { router } = renderAt("/o/acme/projects/new");
  const form = await dialog();
  await userEvent.type(form.getByLabelText("Name"), "Payments service");
  await userEvent.click(form.getByRole("radio", { name: /Copy project/ }));
  expect(form.getByRole("button", { name: "Copy project", hidden: false })).toBeDisabled();
  await userEvent.click(form.getByRole("combobox", { name: "Project to copy" }));
  await userEvent.click(screen.getByRole("option", { name: "Demo" }));
  await userEvent.click(form.getByRole("button", { name: "Copy project" }));
  await waitFor(() => expect(router.state.location.pathname).toBe("/p/acme/payments-service"));
  expect(called(api, "POST", "/api/v1/projects/acme/demo/copy")[0]?.body).toEqual({
    slug: "payments-service",
    name: "Payments service",
    repository_url: null,
  });
  expect(await screen.findByText("Project copied.")).toBeInTheDocument();
});

test("a folder is read into the import tree, dry run first, and the project opens", async () => {
  const report = { constitution_version: "1.0", specs: [], status_fallbacks: [], documents: 3 };
  const api = setup({
    "POST /api/v1/orgs/{org}/projects": () => json(created, 201),
    "POST /api/v1/projects/{org}/{project}/import": () => report,
  });
  const { router } = renderAt("/o/acme/projects/new?start=import");
  const form = await dialog();
  await userEvent.type(form.getByLabelText("Name"), "Payments service");
  expect(form.getByRole("button", { name: "Create and import" })).toBeDisabled();
  await userEvent.upload(form.getByLabelText("Choose folder"), goodTree);
  expect(await form.findByText("repo — specs: 1, documents: 3.")).toBeInTheDocument();
  expect(form.getByText("Not part of an SDD tree and left out: 1.")).toBeInTheDocument();
  await userEvent.click(form.getByRole("button", { name: "Create and import" }));
  await waitFor(() => expect(router.state.location.pathname).toBe("/p/acme/payments-service"));
  const imports = called(api, "POST", "/api/v1/projects/acme/payments-service/import");
  expect(imports.map((c) => (c.body as { dry_run: boolean }).dry_run)).toEqual([true, false]);
  expect(imports[0]?.body).toEqual({
    dry_run: true,
    tree: {
      constitution: {
        relative_path: "docs/constitution.md",
        body: "# Constitution\n\nVersion 1.0\n",
      },
      specs: [
        {
          folder: "001-first",
          number: 1,
          slug: "first",
          documents: [
            ["spec", { relative_path: "specs/001-first/spec.md", body: "Status: approved\n" }],
            ["tasks", { relative_path: "specs/001-first/tasks.md", body: "- [ ] T01 Do it\n" }],
          ],
        },
      ],
      skipped: ["specs/001-first/notes.md"],
      problems: [],
    },
  });
  expect(await screen.findByText("Project created and SDD tree imported.")).toBeInTheDocument();
});

test("problems found in the folder are listed and block the import", async () => {
  const api = setup();
  renderAt("/o/acme/projects/new?start=import");
  const form = await dialog();
  await userEvent.type(form.getByLabelText("Name"), "Payments service");
  await userEvent.upload(form.getByLabelText("Choose folder"), [
    folderFile("specs/first/spec.md", "x"),
    folderFile("specs/2-second/spec.md", "x"),
  ]);
  const problems = await form.findByRole("list", { name: "Problems found" });
  expect(
    within(problems)
      .getAllByRole("listitem")
      .map((li) => li.textContent),
  ).toEqual([
    "specs/2-second: folder name is not a spec branch name such as 001-name",
    "specs/first: folder name is not a spec branch name such as 001-name",
  ]);
  expect(form.getByRole("button", { name: "Create and import" })).toBeDisabled();
  expect(called(api, "POST", "/api/v1/orgs/acme/projects")).toHaveLength(0);
});

test("when the dry run refuses the tree the project is kept and every problem is shown", async () => {
  const api = setup({
    "POST /api/v1/orgs/{org}/projects": () => json(created, 201),
    "POST /api/v1/projects/{org}/{project}/import": () =>
      json(
        {
          code: "validation",
          message:
            'docs/constitution.md: no line starts with "Version "\nspecs/001-first: spec.md is missing',
        },
        422,
      ),
  });
  const { router } = renderAt("/o/acme/projects/new?start=import");
  const form = await dialog();
  await userEvent.type(form.getByLabelText("Name"), "Payments service");
  await userEvent.upload(form.getByLabelText("Choose folder"), goodTree);
  await userEvent.click(await form.findByRole("button", { name: "Create and import" }));
  const problems = await form.findByRole("list", { name: "Problems found" });
  expect(
    within(problems)
      .getAllByRole("listitem")
      .map((li) => li.textContent),
  ).toEqual([
    'docs/constitution.md: no line starts with "Version "',
    "specs/001-first: spec.md is missing",
  ]);
  expect(
    form.getByText(/The project acme\/payments-service was created, but the import found problems/),
  ).toBeInTheDocument();
  expect(router.state.location.pathname).toBe("/o/acme/projects/new");
  expect(called(api, "POST", "/api/v1/projects/acme/payments-service/import")).toHaveLength(1);
  expect(form.getByLabelText("Name")).toBeDisabled();

  await userEvent.upload(form.getByLabelText("Choose another folder"), goodTree);
  await userEvent.click(await form.findByRole("button", { name: "Import" }));
  expect(called(api, "POST", "/api/v1/orgs/acme/projects")).toHaveLength(1);
  expect(called(api, "POST", "/api/v1/projects/acme/payments-service/import")).toHaveLength(2);
});

test("the dialog speaks Spanish", async () => {
  setup();
  renderAt("/o/acme/projects/new", "es");
  const form = within(await screen.findByRole("dialog", { name: "Nuevo proyecto" }));
  expect(form.getByLabelText("Nombre")).toBeInTheDocument();
  expect(form.getByRole("radio", { name: /Importar árbol SDD/ })).toBeInTheDocument();
  expect(form.getByText("Sirve para enlazar los commits de las tareas.")).toBeInTheDocument();
  expect(
    form.getByText("Las constituciones de organización llegarán en una versión posterior."),
  ).toBeInTheDocument();
});
