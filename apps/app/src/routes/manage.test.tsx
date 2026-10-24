import { screen, waitFor, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { afterEach } from "vitest";
import { json, mockApi } from "~/test/api";
import { acme, me } from "~/test/fixtures";
import { renderAt } from "~/test/render";

afterEach(() => {
  vi.unstubAllGlobals();
});

const session = {
  "GET /api/v1/me": () => me,
  "GET /api/v1/orgs": () => ({ organizations: [acme] }),
  "GET /api/v1/orgs/{org}": () => ({ ...acme, created_at: "2026-10-01T10:00:00.000Z" }),
};

async function confirmWith(name: string) {
  const dialog = await screen.findByRole("dialog");
  await userEvent.click(within(dialog).getByRole("button", { name }));
}

function called(api: ReturnType<typeof mockApi>, method: string, path: string) {
  return api.calls.find((c) => c.method === method && c.path === path);
}

test("owners change member roles and remove members", async () => {
  const api = mockApi({
    ...session,
    "GET /api/v1/orgs/{org}/members": () => ({
      members: [
        { handle: "alice", name: "Alice", role: "owner" },
        { handle: "bob", name: "Bob", role: "member" },
      ],
    }),
    "PUT /api/v1/orgs/{org}/members/{handle}": () => new Response(null, { status: 204 }),
    "DELETE /api/v1/orgs/{org}/members/{handle}": () => new Response(null, { status: 204 }),
  });
  renderAt("/o/acme/members");
  const bob = await screen.findByRole("row", { name: /bob/ });
  await userEvent.click(within(bob).getByRole("button", { name: "Make owner" }));
  await waitFor(() =>
    expect(called(api, "PUT", "/api/v1/orgs/acme/members/bob")?.body).toEqual({ role: "owner" }),
  );
  expect(await screen.findByText("Role updated.")).toBeInTheDocument();
  await userEvent.click(within(bob).getByRole("button", { name: "Remove" }));
  expect(called(api, "DELETE", "/api/v1/orgs/acme/members/bob")).toBeUndefined();
  expect(screen.getByRole("dialog", { name: "Remove bob?" })).toBeInTheDocument();
  await confirmWith("Remove");
  await waitFor(() => expect(called(api, "DELETE", "/api/v1/orgs/acme/members/bob")).toBeDefined());
  expect(await screen.findByText("Member removed.")).toBeInTheDocument();
});

test("owners send, resend and revoke invitations", async () => {
  const api = mockApi({
    ...session,
    "GET /api/v1/orgs/{org}/invitations": () => ({
      invitations: [
        {
          id: 4,
          email: "carol@example.com",
          state: "pending",
          inviter: "alice",
          expires_at: "2026-10-29T10:00:00.000Z",
          created_at: "2026-10-22T10:00:00.000Z",
        },
      ],
    }),
    "POST /api/v1/orgs/{org}/invitations": () =>
      json({ id: 5, email: "dan@example.com", expires_at: "2026-10-29T10:00:00.000Z" }, 201),
    "POST /api/v1/orgs/{org}/invitations/{id}/resend": () => new Response(null, { status: 204 }),
    "DELETE /api/v1/orgs/{org}/invitations/{id}": () => new Response(null, { status: 204 }),
  });
  renderAt("/o/acme/invitations");
  await userEvent.type(await screen.findByLabelText("Email address"), "dan@example.com");
  await userEvent.click(screen.getByRole("button", { name: "Send invitation" }));
  await waitFor(() =>
    expect(called(api, "POST", "/api/v1/orgs/acme/invitations")?.body).toEqual({
      email: "dan@example.com",
      language: "en",
    }),
  );
  const carol = screen.getByRole("row", { name: /carol@example.com/ });
  expect(within(carol).getByText("Pending")).toBeInTheDocument();
  expect(await screen.findByText("Invitation sent to dan@example.com.")).toBeInTheDocument();
  await userEvent.click(within(carol).getByRole("button", { name: "Send again" }));
  await waitFor(() =>
    expect(called(api, "POST", "/api/v1/orgs/acme/invitations/4/resend")).toBeDefined(),
  );
  expect(await screen.findByText("Invitation sent again.")).toBeInTheDocument();
  await userEvent.click(within(carol).getByRole("button", { name: "Revoke" }));
  expect(called(api, "DELETE", "/api/v1/orgs/acme/invitations/4")).toBeUndefined();
  await confirmWith("Revoke");
  await waitFor(() =>
    expect(called(api, "DELETE", "/api/v1/orgs/acme/invitations/4")).toBeDefined(),
  );
  expect(await screen.findByText("Invitation revoked.")).toBeInTheDocument();
});

test("the invitation link lets the invited person accept or decline", async () => {
  const api = mockApi({
    ...session,
    "GET /api/v1/invitations/{token}": () => ({
      id: 9,
      organization: "acme",
      organization_name: "Acme Labs",
      inviter: "alice",
      email: me.email,
      state: "pending",
      expires_at: "2026-10-29T10:00:00.000Z",
    }),
    "POST /api/v1/invitations/{token}/accept": () => new Response(null, { status: 204 }),
    "POST /api/v1/invitations/{token}/decline": () => new Response(null, { status: 204 }),
    "GET /api/v1/orgs/{org}/projects": () => ({ projects: [] }),
  });
  const { router } = renderAt("/invitations/inv_token");
  expect(
    await screen.findByRole("heading", { name: "Invitation to Acme Labs" }),
  ).toBeInTheDocument();
  await userEvent.click(screen.getByRole("button", { name: "Accept invitation" }));
  await waitFor(() =>
    expect(called(api, "POST", "/api/v1/invitations/inv_token/accept")).toBeDefined(),
  );
  await waitFor(() => expect(router.state.location.pathname).toBe("/o/acme"));

  const second = renderAt("/invitations/inv_token");
  await userEvent.click(await within(second.container).findByRole("button", { name: "Decline" }));
  await waitFor(() =>
    expect(called(api, "POST", "/api/v1/invitations/inv_token/decline")).toBeDefined(),
  );
  expect(
    await within(second.container).findByText("You declined the invitation."),
  ).toBeInTheDocument();
});

test("owners create projects and manage roles and the repository URL", async () => {
  const project = {
    address: "acme/demo",
    organization: "acme",
    slug: "demo",
    name: "Demo",
    repository_url: null,
    role: "owner",
    created_at: "2026-10-22T10:00:00.000Z",
  };
  const api = mockApi({
    ...session,
    "POST /api/v1/orgs/{org}/projects": () => json(project, 201),
    "GET /api/v1/projects/{org}/{project}": () => project,
    "GET /api/v1/projects/{org}/{project}/members": () => ({
      members: [{ handle: "alice", role: "owner" }],
    }),
    "GET /api/v1/orgs/{org}/members": () => ({
      members: [
        { handle: "alice", name: "Alice", role: "owner" },
        { handle: "bob", name: "Bob", role: "member" },
      ],
    }),
    "PUT /api/v1/projects/{org}/{project}/members/{handle}": () =>
      new Response(null, { status: 204 }),
    "PUT /api/v1/projects/{org}/{project}/repository-url": () =>
      new Response(null, { status: 204 }),
    "DELETE /api/v1/projects/{org}/{project}/repository-url": () =>
      new Response(null, { status: 204 }),
  });
  const { router } = renderAt("/o/acme/projects/new");
  await userEvent.type(await screen.findByLabelText("Project address"), "demo");
  await userEvent.type(screen.getByLabelText("Project name"), "Demo");
  await userEvent.click(screen.getByRole("button", { name: "Create project" }));
  await waitFor(() =>
    expect(called(api, "POST", "/api/v1/orgs/acme/projects")?.body).toEqual({
      slug: "demo",
      name: "Demo",
    }),
  );
  await waitFor(() => expect(router.state.location.pathname).toBe("/p/acme/demo/settings"));

  await userEvent.selectOptions(await screen.findByLabelText("Person"), "bob");
  await userEvent.selectOptions(screen.getByLabelText("Role"), "editor");
  await userEvent.click(screen.getByRole("button", { name: "Grant role" }));
  await waitFor(() =>
    expect(called(api, "PUT", "/api/v1/projects/acme/demo/members/bob")?.body).toEqual({
      role: "editor",
    }),
  );
  await userEvent.type(screen.getByLabelText("Repository URL"), "https://github.com/acme/demo");
  await userEvent.click(screen.getByRole("button", { name: "Save" }));
  await waitFor(() =>
    expect(called(api, "PUT", "/api/v1/projects/acme/demo/repository-url")?.body).toEqual({
      url: "https://github.com/acme/demo",
    }),
  );
});

test("the profile lists authorized agents and revokes them", async () => {
  const api = mockApi({
    ...session,
    "GET /api/v1/me/clients": () => ({
      clients: [
        {
          client_id: "claude-code-1",
          name: "Claude Code",
          scopes: ["sdd-vault-mcp"],
          granted_at: "2026-10-20T10:00:00.000Z",
          updated_at: "2026-10-21T10:00:00.000Z",
        },
      ],
    }),
    "DELETE /api/v1/me/clients/{client_id}": () => new Response(null, { status: 204 }),
  });
  renderAt("/profile");
  const agent = await screen.findByRole("listitem", { name: /Claude Code/ });
  await userEvent.click(within(agent).getByRole("button", { name: "Revoke access" }));
  expect(called(api, "DELETE", "/api/v1/me/clients/claude-code-1")).toBeUndefined();
  await confirmWith("Revoke access");
  await waitFor(() =>
    expect(called(api, "DELETE", "/api/v1/me/clients/claude-code-1")).toBeDefined(),
  );
  expect(await screen.findByText("Access revoked.")).toBeInTheDocument();
});

test("no screen offers to write specs, documents, statuses or the constitution", async () => {
  mockApi({
    ...session,
    "GET /api/v1/projects/{org}/{project}": () => ({
      address: "acme/demo",
      organization: "acme",
      slug: "demo",
      name: "Demo",
      repository_url: null,
      role: "owner",
      created_at: "2026-10-22T10:00:00.000Z",
    }),
    "GET /api/v1/projects/{org}/{project}/status": () => ({
      constitution_version: null,
      specs: [],
      active: null,
    }),
    "GET /api/v1/projects/{org}/{project}/members": () => ({ members: [] }),
  });
  renderAt("/p/acme/demo");
  expect(await screen.findByRole("heading", { name: "Demo" })).toBeInTheDocument();
  for (const label of [/new spec/i, /edit/i, /write/i, /approve/i, /archive/i, /status/i]) {
    expect(screen.queryByRole("button", { name: label })).toBeNull();
  }
  expect(document.querySelector("textarea")).toBeNull();
});
