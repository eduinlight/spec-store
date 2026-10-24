import { screen, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { afterEach } from "vitest";
import { mockApi } from "~/test/api";
import { acme, me, projectDetail, specCard } from "~/test/fixtures";
import { renderAt } from "~/test/render";

afterEach(() => {
  vi.unstubAllGlobals();
});

function specsApi(cards: ReturnType<typeof specCard>[], project = projectDetail()) {
  return mockApi({
    "GET /api/v1/me": () => me,
    "GET /api/v1/orgs": () => ({ organizations: [acme] }),
    "GET /api/v1/orgs/{org}/projects": () => ({ projects: [] }),
    "GET /api/v1/projects/{org}/{project}": () => project,
    "GET /api/v1/projects/{org}/{project}/status": () => ({
      constitution_version: null,
      specs: [],
      active: null,
    }),
    "GET /api/v1/projects/{org}/{project}/specs/cards": () => ({ specs: cards }),
  });
}

const mixed = () => [
  specCard(1, { title: "First spec", slug: "first", status: "archived" }),
  specCard(2, { title: "Billing rules", slug: "billing", status: "approved" }),
  specCard(3, { title: "Refund flow", slug: "refunds", status: "clarified", open_markers: 2 }),
  specCard(4, {
    title: "Diesel migration",
    slug: "diesel-orm",
    status: "draft",
    updated_at: "2026-10-30T10:00:00.000Z",
  }),
];

const cardTitles = () =>
  within(screen.getByRole("list", { name: "Specs" }))
    .getAllByRole("listitem")
    .map((item) => within(item).getByRole("link").getAttribute("href"));

test("the specs page lists cards newest first with counts on the status chips", async () => {
  specsApi(mixed());
  renderAt("/p/acme/demo/specs");
  expect(await screen.findByRole("heading", { name: "Specs" })).toBeInTheDocument();
  expect(screen.getByText("4 specs")).toBeInTheDocument();
  expect(cardTitles()).toEqual([
    "/p/acme/demo/specs/4",
    "/p/acme/demo/specs/3",
    "/p/acme/demo/specs/2",
    "/p/acme/demo/specs/1",
  ]);
  const card = screen.getByRole("link", { name: /003.*Refund flow/ });
  expect(within(card).getByText("Clarified")).toBeInTheDocument();
  expect(within(card).getByText("2 open questions")).toBeInTheDocument();
  expect(within(card).getByText("Summary 3")).toBeInTheDocument();
  expect(within(card).getByText("refunds")).toBeInTheDocument();
  expect(within(card).getByText("Sep 2026")).toBeInTheDocument();
  expect(within(card).getByText("4 requirements")).toBeInTheDocument();
  expect(within(card).getByText("2 / 5 tasks")).toBeInTheDocument();
  expect(screen.getByRole("button", { name: "All 4" })).toHaveAttribute("aria-pressed", "true");
  expect(screen.getByRole("button", { name: "Active 3" })).toBeInTheDocument();
  expect(screen.getByRole("button", { name: "Draft 1" })).toBeInTheDocument();
  expect(screen.getByRole("button", { name: "Clarified 1" })).toBeInTheDocument();
  expect(screen.getByRole("button", { name: "Approved 1" })).toBeInTheDocument();
  expect(screen.getByRole("button", { name: "Archived 1" })).toBeInTheDocument();
});

test("a status chip narrows the list and the active chip hides archived specs", async () => {
  specsApi(mixed());
  renderAt("/p/acme/demo/specs");
  await screen.findByRole("heading", { name: "Specs" });
  await userEvent.click(screen.getByRole("button", { name: "Active 3" }));
  expect(cardTitles()).toEqual([
    "/p/acme/demo/specs/4",
    "/p/acme/demo/specs/3",
    "/p/acme/demo/specs/2",
  ]);
  await userEvent.click(screen.getByRole("button", { name: "Archived 1" }));
  expect(cardTitles()).toEqual(["/p/acme/demo/specs/1"]);
  expect(screen.getByText("Showing 1 of 1")).toBeInTheDocument();
});

test("search matches the title, the slug and the number", async () => {
  specsApi(mixed());
  renderAt("/p/acme/demo/specs");
  await screen.findByRole("heading", { name: "Specs" });
  const search = screen.getByRole("textbox", { name: "Search by title, slug or number" });
  await userEvent.type(search, "refund");
  expect(cardTitles()).toEqual(["/p/acme/demo/specs/3"]);
  await userEvent.clear(search);
  await userEvent.type(search, "diesel-orm");
  expect(cardTitles()).toEqual(["/p/acme/demo/specs/4"]);
  await userEvent.clear(search);
  await userEvent.type(search, "002");
  expect(cardTitles()).toEqual(["/p/acme/demo/specs/2"]);
  await userEvent.clear(search);
  await userEvent.type(search, "nothing here");
  expect(screen.getByText("No specs match these filters.")).toBeInTheDocument();
});

test("the sort offers oldest first and recently updated", async () => {
  specsApi(mixed());
  renderAt("/p/acme/demo/specs");
  await screen.findByRole("heading", { name: "Specs" });
  await userEvent.click(screen.getByRole("combobox", { name: "Sort specs" }));
  await userEvent.click(await screen.findByRole("option", { name: "Number, oldest first" }));
  expect(cardTitles()).toEqual([
    "/p/acme/demo/specs/1",
    "/p/acme/demo/specs/2",
    "/p/acme/demo/specs/3",
    "/p/acme/demo/specs/4",
  ]);
  await userEvent.click(screen.getByRole("combobox", { name: "Sort specs" }));
  await userEvent.click(await screen.findByRole("option", { name: "Recently updated" }));
  expect(cardTitles()[0]).toBe("/p/acme/demo/specs/4");
});

test("twenty cards are shown at a time with Load more and the showing count", async () => {
  const many = Array.from({ length: 45 }, (_, index) => specCard(index + 1));
  specsApi(many);
  renderAt("/p/acme/demo/specs");
  await screen.findByRole("heading", { name: "Specs" });
  expect(cardTitles()).toHaveLength(20);
  expect(screen.getByText("Showing 20 of 45")).toBeInTheDocument();
  await userEvent.click(screen.getByRole("button", { name: "Load 20 more" }));
  expect(cardTitles()).toHaveLength(40);
  await userEvent.click(screen.getByRole("button", { name: "Load 5 more" }));
  expect(cardTitles()).toHaveLength(45);
  expect(screen.getByText("Showing 45 of 45")).toBeInTheDocument();
  expect(screen.queryByRole("button", { name: /Load/ })).not.toBeInTheDocument();
});

test("a card opens its spec without leaving the app", async () => {
  specsApi(mixed());
  const { router } = renderAt("/p/acme/demo/specs");
  await screen.findByRole("heading", { name: "Specs" });
  await userEvent.click(screen.getByRole("link", { name: /002.*Billing rules/ }));
  expect(router.state.location.pathname).toBe("/p/acme/demo/specs/2");
});

test("an archived project shows the read-only banner and a project without specs says so", async () => {
  specsApi([], projectDetail({ archived: true }));
  renderAt("/p/acme/demo/specs");
  expect(await screen.findByText(/archived and read-only/)).toBeInTheDocument();
  expect(screen.getByText("No specs yet")).toBeInTheDocument();
});

test("the specs page speaks Spanish", async () => {
  specsApi(mixed());
  renderAt("/p/acme/demo/specs", "es");
  expect(await screen.findByRole("button", { name: "Todas 4" })).toBeInTheDocument();
  expect(screen.getByText("4 specs")).toBeInTheDocument();
  expect(
    screen.getByRole("textbox", { name: "Buscar por título, slug o número" }),
  ).toBeInTheDocument();
  expect(screen.getByText("Mostrando 4 de 4")).toBeInTheDocument();
  expect(screen.getByRole("button", { name: "Aprobadas 1" })).toBeInTheDocument();
});
