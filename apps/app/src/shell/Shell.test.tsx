import { screen, waitFor, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { afterEach } from "vitest";
import { mockApi } from "~/test/api";
import { acme, demoProject, me } from "~/test/fixtures";
import { renderAt } from "~/test/render";

afterEach(() => {
  vi.unstubAllGlobals();
});

function shellApi(extra: Record<string, () => unknown> = {}) {
  return mockApi({
    "GET /api/v1/me": () => me,
    "GET /api/v1/orgs": () => ({
      organizations: [acme, { slug: "beta", name: "Beta Inc", role: "member" }],
    }),
    "GET /api/v1/orgs/{org}/projects": () => ({
      projects: [demoProject, { address: "acme/site", slug: "site", name: "Site", role: "reader" }],
    }),
    "GET /api/v1/projects/{org}/{project}/status": () => ({
      constitution_version: "1.0",
      specs: [
        {
          number: 5,
          slug: "saas-foundation",
          branch: "005-saas-foundation",
          kinds: [],
          status: "approved",
        },
      ],
      active: {
        number: 5,
        slug: "saas-foundation",
        branch: "005-saas-foundation",
        status: "approved",
        markers: 0,
        tasks_done: 1,
        tasks_total: 2,
      },
    }),
    "PUT /api/v1/me/language": () => new Response(null, { status: 204 }),
    ...extra,
  });
}

test("every signed-in screen is wrapped in the header, the sidebar and the user menu", async () => {
  shellApi();
  renderAt("/p/acme/demo/activity");
  expect(await screen.findByRole("heading", { name: "Activity" })).toBeInTheDocument();
  expect(screen.getByRole("banner")).toBeInTheDocument();
  expect(screen.getByRole("button", { name: "Project" })).toHaveTextContent("demo");
  const nav = screen.getByRole("navigation", { name: "Main navigation" });
  expect(within(nav).getByRole("link", { name: "Activity" })).toHaveAttribute(
    "aria-current",
    "page",
  );
  expect(screen.getByRole("button", { name: "User menu" })).toHaveTextContent("@alice");
  expect(screen.getByRole("main")).toBeInTheDocument();
});

test("the project switcher shows the active spec of the current project", async () => {
  shellApi();
  renderAt("/p/acme/demo/activity");
  await screen.findByRole("heading", { name: "Activity" });
  await userEvent.click(screen.getByRole("button", { name: "Project" }));
  expect(screen.getByRole("option", { name: /demo/ })).toHaveTextContent("005 saas-foundation");
});

test("every upcoming screen shows an empty state naming what is coming", async () => {
  shellApi();
  const screens: [string, string, RegExp][] = [
    ["/p/acme/demo/active", "Active specs", /specs your team is working on/],
    ["/p/acme/demo/specs", "Specs", /full list of specs/],
    ["/p/acme/demo/activity", "Activity", /feed of what people and agents change/],
    ["/p/acme/demo/members", "Project members", /project settings/],
    ["/account/tokens", "My tokens", /tokens your agents use/],
    ["/account/preferences", "Preferences", /time zone/],
    ["/o/acme/security", "Security", /Sign-in policies/],
    ["/o/acme/billing", "Billing & usage", /invoices/],
  ];
  for (const [path, title, body] of screens) {
    const { unmount } = renderAt(path);
    expect(await screen.findByRole("heading", { level: 1, name: title })).toBeInTheDocument();
    expect(screen.getByText(body)).toBeInTheDocument();
    expect(screen.getByText("Coming soon")).toBeInTheDocument();
    unmount();
  }
});

test("opening an account item switches the sidebar and the way back returns to the page", async () => {
  shellApi();
  const { router } = renderAt("/p/acme/demo/activity");
  await screen.findByRole("heading", { name: "Activity" });
  await userEvent.click(screen.getByRole("button", { name: "User menu" }));
  await userEvent.click(screen.getByRole("menuitem", { name: "My tokens" }));
  expect(await screen.findByRole("heading", { level: 1, name: "My tokens" })).toBeInTheDocument();
  const nav = screen.getByRole("navigation", { name: "Main navigation" });
  expect(within(nav).getByText("Account")).toBeInTheDocument();
  expect(within(nav).getByRole("link", { name: "Back to demo" })).toBeInTheDocument();
  expect(screen.getByRole("button", { name: "Project" })).toHaveTextContent("demo");
  await userEvent.click(within(nav).getByRole("link", { name: "Back to demo" }));
  expect(await screen.findByRole("heading", { level: 1, name: "Activity" })).toBeInTheDocument();
  expect(router.state.location.pathname).toBe("/p/acme/demo/activity");
  expect(within(screen.getByRole("navigation")).queryByText("Account")).not.toBeInTheDocument();
});

test("switching project keeps the same kind of screen", async () => {
  shellApi();
  const { router } = renderAt("/p/acme/demo/activity");
  await screen.findByRole("heading", { name: "Activity" });
  await userEvent.click(screen.getByRole("button", { name: "Project" }));
  await userEvent.click(screen.getByRole("option", { name: /site/ }));
  await waitFor(() => expect(router.state.location.pathname).toBe("/p/acme/site/activity"));
});

test("switching organization from an organization screen keeps the screen", async () => {
  shellApi({
    "GET /api/v1/orgs/{org}": () => ({ ...acme, created_at: "2026-10-01T10:00:00.000Z" }),
    "GET /api/v1/orgs/{org}/members": () => ({ members: [] }),
  });
  const { router } = renderAt("/o/acme/members");
  await screen.findByRole("heading", { name: /Members of/ });
  await userEvent.click(screen.getByRole("button", { name: "Organization" }));
  await userEvent.click(screen.getByRole("option", { name: /Beta Inc/ }));
  await waitFor(() => expect(router.state.location.pathname).toBe("/o/beta/members"));
});

test("the language choice in the user menu switches the texts and is saved", async () => {
  const api = shellApi();
  renderAt("/p/acme/demo/activity");
  await screen.findByRole("heading", { name: "Activity" });
  await userEvent.click(screen.getByRole("button", { name: "User menu" }));
  await userEvent.click(screen.getByRole("menuitemradio", { name: "Español" }));
  expect(await screen.findByRole("heading", { level: 1, name: "Actividad" })).toBeInTheDocument();
  await waitFor(() =>
    expect(api.calls.find((call) => call.method === "PUT")?.body).toEqual({ language: "es" }),
  );
});

test("sign out posts the logout and leaves through the returned address", async () => {
  const assign = vi.fn();
  vi.stubGlobal("location", { ...window.location, assign, origin: "http://app.test" });
  const api = shellApi({
    "POST /auth/logout": () => ({ logout_url: "http://idp.test/logout" }),
  });
  renderAt("/p/acme/demo/activity");
  await screen.findByRole("heading", { name: "Activity" });
  await userEvent.click(screen.getByRole("button", { name: "User menu" }));
  await userEvent.click(screen.getByRole("menuitem", { name: "Sign out" }));
  await waitFor(() => expect(assign).toHaveBeenCalledWith("http://idp.test/logout"));
  expect(api.calls.some((call) => call.method === "POST" && call.path === "/auth/logout")).toBe(
    true,
  );
});

test("the notifications button opens an empty drawer", async () => {
  shellApi();
  renderAt("/p/acme/demo/activity");
  await screen.findByRole("heading", { name: "Activity" });
  await userEvent.click(screen.getByRole("button", { name: "Notifications" }));
  const drawer = screen.getByRole("dialog", { name: "Notifications" });
  expect(within(drawer).getByText("Nothing waiting for you.")).toBeInTheDocument();
  await userEvent.click(within(drawer).getByRole("button", { name: "Close" }));
  expect(screen.queryByRole("dialog")).not.toBeInTheDocument();
});
