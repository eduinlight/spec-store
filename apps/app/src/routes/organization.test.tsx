import { screen, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { afterEach } from "vitest";
import { mockApi } from "~/test/api";
import { acme, me, summary } from "~/test/fixtures";
import { renderAt } from "~/test/render";

afterEach(() => {
  vi.unstubAllGlobals();
});

const payments = summary({
  address: "acme/payments",
  slug: "payments",
  name: "Payments",
  constitution_version: "3.0",
  constitution_updated_at: "2026-10-20T10:00:00.000Z",
  active_spec: { number: 5, slug: "deploy", status: "approved", tasks_done: 3, tasks_total: 12 },
  last_activity: { actor: "rita", client: "Claude Code", at: "2026-10-24T09:00:00.000Z" },
  open_markers: 2,
});
const draftOnly = summary({ address: "acme/blank", slug: "blank", name: "Blank" });
const others = summary({ address: "acme/others", slug: "others", name: "Others", role: null });
const old = summary({ address: "acme/old", slug: "old", name: "Old", archived: true });

function setup(projects: unknown[], organization = acme) {
  return mockApi({
    "GET /api/v1/me": () => me,
    "GET /api/v1/orgs": () => ({ organizations: [organization] }),
    "GET /api/v1/orgs/{org}": () => ({ ...organization, created_at: "2026-10-01T10:00:00.000Z" }),
    "GET /api/v1/orgs/{org}/projects": () => ({ projects }),
  });
}

const cards = () => within(screen.getByRole("list", { name: "Projects" }));

test("a project card shows its role, constitution, active spec, progress, last activity and markers", async () => {
  setup([payments]);
  renderAt("/o/acme");
  const link = await screen.findByRole("link", { name: /Payments/ });
  expect(link).toHaveAttribute("href", "/p/acme/payments");
  const card = within(link);
  expect(card.getByText("Owner")).toBeInTheDocument();
  expect(card.getByText(/Constitution v3\.0/)).toBeInTheDocument();
  expect(card.getByText("005")).toBeInTheDocument();
  expect(card.getByText("deploy")).toBeInTheDocument();
  expect(card.getByText("Approved")).toBeInTheDocument();
  expect(card.getByRole("progressbar", { name: "Task progress" })).toHaveAttribute(
    "aria-valuenow",
    "25",
  );
  expect(card.getByText("3/12")).toBeInTheDocument();
  expect(card.getByText(/rita/)).toBeInTheDocument();
  expect(card.getByText("Claude Code")).toBeInTheDocument();
  await userEvent.hover(card.getByText("? 2 open"));
  expect(await screen.findByRole("tooltip")).toHaveTextContent(
    "2 open questions in the active spec",
  );
});

test("a project without constitution, specs or activity says so", async () => {
  setup([draftOnly]);
  renderAt("/o/acme");
  const link = await screen.findByRole("link", { name: /Blank/ });
  expect(within(link).getByText("No constitution yet")).toBeInTheDocument();
  expect(within(link).getByText("No specs yet")).toBeInTheDocument();
  expect(within(link).getByText("No activity yet")).toBeInTheDocument();
  expect(within(link).queryByText(/open$/)).toBeNull();
});

test("My projects lists the projects the caller belongs to, All the unarchived ones, Archived the rest", async () => {
  setup([payments, others, old]);
  renderAt("/o/acme");
  await screen.findByRole("link", { name: /Payments/ });
  expect(cards().queryByRole("link", { name: /Others/ })).toBeNull();
  expect(cards().queryByRole("link", { name: /Old/ })).toBeNull();

  await userEvent.click(screen.getByRole("tab", { name: "All" }));
  expect(cards().getByRole("link", { name: /Others/ })).toBeInTheDocument();
  expect(cards().getByRole("link", { name: /Payments/ })).toBeInTheDocument();
  expect(cards().queryByRole("link", { name: /Old/ })).toBeNull();

  await userEvent.click(screen.getByRole("tab", { name: "Archived" }));
  const archived = cards().getByRole("link", { name: /Old/ });
  expect(within(archived).getByText("Archived")).toBeInTheDocument();
  expect(cards().queryByRole("link", { name: /Payments/ })).toBeNull();
});

test("each filter has its own empty state", async () => {
  setup([others]);
  renderAt("/o/acme");
  expect(await screen.findByText("You are not a member of any project yet")).toBeInTheDocument();
  await userEvent.click(screen.getByRole("tab", { name: "Archived" }));
  expect(screen.getByText("No archived projects")).toBeInTheDocument();
  expect(
    screen.getByText("Archived projects stay read-only and keep their full history."),
  ).toBeInTheDocument();
});

test("the All filter of an organization without projects invites to create the first one", async () => {
  setup([]);
  renderAt("/o/acme");
  await userEvent.click(await screen.findByRole("tab", { name: "All" }));
  expect(screen.getByText("This organization has no projects yet")).toBeInTheDocument();
});

test("owners see the create and import actions, members do not", async () => {
  setup([payments]);
  const owner = renderAt("/o/acme");
  expect(await screen.findByRole("button", { name: "Create project" })).toBeInTheDocument();
  expect(screen.getByRole("button", { name: "Import SDD tree" })).toBeInTheDocument();
  owner.unmount();
  vi.unstubAllGlobals();

  setup([payments], { ...acme, role: "member" });
  renderAt("/o/acme");
  await screen.findByRole("link", { name: /Payments/ });
  expect(screen.queryByRole("button", { name: "Create project" })).toBeNull();
  expect(screen.queryByRole("button", { name: "Import SDD tree" })).toBeNull();
});

test("the cards speak Spanish", async () => {
  setup([payments]);
  renderAt("/o/acme", "es");
  const link = await screen.findByRole("link", { name: /Payments/ });
  expect(within(link).getByText(/Constitución v3\.0/)).toBeInTheDocument();
  expect(within(link).getByText("? 2 abiertas")).toBeInTheDocument();
  expect(screen.getByRole("tab", { name: "Mis proyectos" })).toBeInTheDocument();
});
