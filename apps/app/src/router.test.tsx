import { screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { afterEach } from "vitest";
import { json, mockApi } from "~/test/api";
import { acme, demoProject, me } from "~/test/fixtures";
import { renderAt } from "~/test/render";
import { requireSession } from "./lib/session";

afterEach(() => {
  vi.unstubAllGlobals();
});

test("a 401 from the API sends the browser to sign-in and back to the requested screen", async () => {
  mockApi({
    "GET /api/v1/me": () =>
      json({ code: "unauthenticated", message: "authentication required" }, 401),
  });
  let thrown: unknown;
  try {
    await requireSession(new Request("http://app.test/p/acme/demo?tab=x"));
  } catch (error) {
    thrown = error;
  }
  expect(thrown).toBeInstanceOf(Response);
  const response = thrown as Response;
  expect(response.status).toBe(302);
  expect(response.headers.get("Location")).toBe(
    "/auth/login?return_to=%2Fp%2Facme%2Fdemo%3Ftab%3Dx",
  );
  expect(response.headers.get("X-Remix-Reload-Document")).toBe("true");
});

test("a person without organizations sees the onboarding form and their invitations", async () => {
  const api = mockApi({
    "GET /api/v1/me": () => me,
    "GET /api/v1/orgs": () => ({ organizations: [] }),
    "GET /api/v1/invitations": () => ({
      invitations: [
        {
          id: 7,
          organization: "beta",
          organization_name: "Beta Inc",
          inviter: "bob",
          email: me.email,
          state: "pending",
          expires_at: "2026-10-29T10:00:00.000Z",
        },
      ],
    }),
    "POST /api/v1/orgs": () =>
      json({ slug: "acme", name: "Acme Labs", role: "owner", created_at: "x" }, 201),
    "GET /api/v1/orgs/{org}": () => ({ ...acme, created_at: "2026-10-22T10:00:00.000Z" }),
    "GET /api/v1/orgs/{org}/projects": () => ({ projects: [] }),
  });
  renderAt("/");
  expect(await screen.findByRole("heading", { name: "Welcome to SDD Vault" })).toBeInTheDocument();
  expect(screen.getByText("Beta Inc")).toBeInTheDocument();
  expect(screen.getByText("Invited by bob")).toBeInTheDocument();
  await userEvent.type(screen.getByLabelText("Organization address"), "acme");
  await userEvent.type(screen.getByLabelText("Organization name"), "Acme Labs");
  await userEvent.click(screen.getByRole("button", { name: "Create organization" }));
  await waitFor(() =>
    expect(api.calls.some((c) => c.method === "POST" && c.path === "/api/v1/orgs")).toBe(true),
  );
  expect(api.calls.find((c) => c.method === "POST")?.body).toEqual({
    slug: "acme",
    name: "Acme Labs",
  });
  expect(await screen.findByRole("heading", { name: "Acme Labs" })).toBeInTheDocument();
});

test("a member lands on an organization, sees its projects and can switch organizations", async () => {
  mockApi({
    "GET /api/v1/me": () => me,
    "GET /api/v1/orgs": () => ({
      organizations: [acme, { slug: "beta", name: "Beta Inc", role: "member" }],
    }),
    "GET /api/v1/orgs/{org}": (_r, p) =>
      p.org === "acme"
        ? { ...acme, created_at: "2026-10-22T10:00:00.000Z" }
        : {
            slug: "beta",
            name: "Beta Inc",
            role: "member",
            created_at: "2026-10-22T10:00:00.000Z",
          },
    "GET /api/v1/orgs/{org}/projects": (_r, p) =>
      p.org === "acme"
        ? { projects: [demoProject] }
        : { projects: [{ address: "beta/site", slug: "site", name: "Site", role: "reader" }] },
  });
  const { router } = renderAt("/");
  expect(await screen.findByRole("heading", { name: "Acme Labs" })).toBeInTheDocument();
  expect(screen.getByRole("link", { name: "Demo" })).toHaveAttribute("href", "/p/acme/demo");
  await userEvent.click(screen.getByRole("button", { name: "Organization" }));
  await userEvent.click(screen.getByRole("option", { name: /Beta Inc/ }));
  expect(await screen.findByRole("heading", { name: "Beta Inc" })).toBeInTheDocument();
  expect(screen.getByRole("link", { name: "Site" })).toBeInTheDocument();
  expect(router.state.location.pathname).toBe("/o/beta");
});
