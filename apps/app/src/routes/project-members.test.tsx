import { screen, waitFor, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { afterEach } from "vitest";
import { json, mockApi } from "~/test/api";
import { acme, me, projectDetail, summary } from "~/test/fixtures";
import { renderAt } from "~/test/render";

afterEach(() => {
  vi.unstubAllGlobals();
});

const NO_CONTENT = () => new Response(null, { status: 204 });

const members = [
  {
    handle: "alice",
    name: "Alice",
    role: "owner",
    guest: false,
    joined_at: "2026-10-01T10:00:00.000Z",
    last_active_at: new Date(Date.now() - 3_600_000).toISOString(),
  },
  {
    handle: "bob",
    name: "Bob Builder",
    role: "editor",
    guest: false,
    joined_at: "2026-10-02T10:00:00.000Z",
    last_active_at: null,
  },
  {
    handle: "gia",
    name: "Gia Guest",
    role: "reader",
    guest: true,
    joined_at: "2026-10-03T10:00:00.000Z",
    last_active_at: null,
  },
];

const pending = {
  id: 7,
  email: "carol@example.com",
  role: "editor",
  state: "pending",
  inviter: "alice",
  expires_at: "2026-10-29T10:00:00.000Z",
  created_at: "2026-10-22T10:00:00.000Z",
};

const revoked = { ...pending, id: 8, email: "old@example.com", state: "revoked" };

function setup(extra: Record<string, Parameters<typeof mockApi>[0][string]> = {}, role = "owner") {
  return mockApi({
    "GET /api/v1/me": () => me,
    "GET /api/v1/orgs": () => ({ organizations: [{ ...acme, guest: false }] }),
    "GET /api/v1/orgs/{org}": () => ({ ...acme, created_at: "2026-10-01T10:00:00.000Z" }),
    "GET /api/v1/orgs/{org}/projects": () => ({ projects: [summary({ role })] }),
    "GET /api/v1/projects/{org}/{project}": () => projectDetail({ role }),
    "GET /api/v1/projects/{org}/{project}/status": () => ({
      constitution_version: null,
      specs: [],
      active: null,
    }),
    "GET /api/v1/projects/{org}/{project}/members": () => ({ members }),
    "GET /api/v1/projects/{org}/{project}/members/invitations": () => ({
      invitations: [pending, revoked],
    }),
    ...extra,
  });
}

function called(api: ReturnType<typeof mockApi>, method: string, path: string) {
  return api.calls.find((c) => c.method === method && c.path === path);
}

async function confirmWith(name: string) {
  const dialog = await screen.findByRole("dialog");
  await userEvent.click(within(dialog).getByRole("button", { name }));
}

const BASE = "/api/v1/projects/acme/demo/members";

test("members are listed with role, guest tag, joined and last active, owners first", async () => {
  setup();
  renderAt("/p/acme/demo/members");
  expect(await screen.findByRole("heading", { name: "Members" })).toBeInTheDocument();
  const bob = await screen.findByRole("row", { name: /Bob Builder/ });
  expect(within(bob).getByText("@bob")).toBeInTheDocument();
  expect(within(bob).getByText("—")).toBeInTheDocument();
  expect(within(bob).queryByText("Guest")).toBeNull();
  const gia = screen.getByRole("row", { name: /Gia Guest/ });
  expect(within(gia).getByText("Guest")).toBeInTheDocument();
  const alice = screen.getByRole("row", { name: /Alice/ });
  expect(within(alice).getByText(/ago/)).toBeInTheDocument();
});

test("owners change a role with the select", async () => {
  const api = setup({ "PUT /api/v1/projects/{org}/{project}/members/{handle}": NO_CONTENT });
  renderAt("/p/acme/demo/members");
  const bob = await screen.findByRole("row", { name: /Bob Builder/ });
  await userEvent.click(within(bob).getByRole("combobox", { name: "Role of Bob Builder" }));
  await userEvent.click(await screen.findByRole("option", { name: "Reader" }));
  await waitFor(() => expect(called(api, "PUT", `${BASE}/bob`)?.body).toEqual({ role: "reader" }));
  expect(await screen.findByText("Role updated.")).toBeInTheDocument();
});

test("the server's last owner refusal is shown as an error toast", async () => {
  setup({
    "PUT /api/v1/projects/{org}/{project}/members/{handle}": () =>
      json({ code: "unprocessable", message: "a project needs at least one owner" }, 422),
  });
  renderAt("/p/acme/demo/members");
  const bob = await screen.findByRole("row", { name: /Bob Builder/ });
  await userEvent.click(within(bob).getByRole("combobox", { name: "Role of Bob Builder" }));
  await userEvent.click(await screen.findByRole("option", { name: "Reader" }));
  expect(await screen.findByText("a project needs at least one owner")).toBeInTheDocument();
});

test("the only owner cannot change or remove themself", async () => {
  setup();
  renderAt("/p/acme/demo/members");
  const alice = await screen.findByRole("row", { name: /Alice/ });
  expect(within(alice).queryByRole("combobox")).toBeNull();
  expect(within(alice).getByText("Owner")).toBeInTheDocument();
  expect(within(alice).queryByRole("button", { name: /^Remove/ })).toBeNull();
});

test("removing a member asks for confirmation and then shows a toast", async () => {
  const api = setup({
    "DELETE /api/v1/projects/{org}/{project}/members/{handle}": NO_CONTENT,
  });
  renderAt("/p/acme/demo/members");
  const gia = await screen.findByRole("row", { name: /Gia Guest/ });
  await userEvent.click(within(gia).getByRole("button", { name: "Remove Gia Guest" }));
  expect(called(api, "DELETE", `${BASE}/gia`)).toBeUndefined();
  expect(screen.getByRole("dialog", { name: "Remove Gia Guest?" })).toBeInTheDocument();
  await confirmWith("Remove");
  await waitFor(() => expect(called(api, "DELETE", `${BASE}/gia`)).toBeDefined());
  expect(await screen.findByText("Member removed.")).toBeInTheDocument();
});

test("inviting an organization member adds them at once", async () => {
  const api = setup({
    "POST /api/v1/projects/{org}/{project}/members/invitations": () => ({ added: "bea" }),
  });
  renderAt("/p/acme/demo/members");
  await userEvent.click(await screen.findByRole("button", { name: "Invite" }));
  const dialog = within(await screen.findByRole("dialog", { name: "Invite to Demo" }));
  expect(
    dialog.getByText(
      "People who are not members of Acme Labs join as guests and see only this project.",
    ),
  ).toBeInTheDocument();
  expect(dialog.getByText("Readers are free on paid plans.")).toBeInTheDocument();
  await userEvent.type(dialog.getByLabelText("Email"), "bea@acme.com");
  await userEvent.click(dialog.getByRole("combobox", { name: "Role" }));
  await userEvent.click(await screen.findByRole("option", { name: "Editor" }));
  await userEvent.click(dialog.getByRole("button", { name: "Send invite" }));
  await waitFor(() =>
    expect(called(api, "POST", `${BASE}/invitations`)?.body).toEqual({
      email: "bea@acme.com",
      role: "editor",
      language: "en",
    }),
  );
  expect(await screen.findByText("Added @bea as Editor")).toBeInTheDocument();
  await waitFor(() => expect(screen.queryByRole("dialog")).toBeNull());
});

test("inviting an outside email records a pending invitation", async () => {
  const api = setup({
    "POST /api/v1/projects/{org}/{project}/members/invitations": () =>
      json({ invited: "guest@example.test", id: 9 }, 201),
  });
  renderAt("/p/acme/demo/members");
  await userEvent.click(await screen.findByRole("button", { name: "Invite" }));
  const dialog = within(await screen.findByRole("dialog", { name: "Invite to Demo" }));
  await userEvent.type(dialog.getByLabelText("Email"), "guest@example.test");
  await userEvent.click(dialog.getByRole("button", { name: "Send invite" }));
  await waitFor(() =>
    expect(called(api, "POST", `${BASE}/invitations`)?.body).toEqual({
      email: "guest@example.test",
      role: "reader",
      language: "en",
    }),
  );
  expect(await screen.findByText("Invitation sent to guest@example.test")).toBeInTheDocument();
});

test("the invite dialog validates the email and shows server errors inline", async () => {
  const api = setup({
    "POST /api/v1/projects/{org}/{project}/members/invitations": () =>
      json({ code: "conflict", message: "bob is already a member of this project" }, 409),
  });
  renderAt("/p/acme/demo/members");
  await userEvent.click(await screen.findByRole("button", { name: "Invite" }));
  const dialog = within(await screen.findByRole("dialog", { name: "Invite to Demo" }));
  await userEvent.type(dialog.getByLabelText("Email"), "not-an-email");
  expect(dialog.getByText("Enter a valid email address.")).toBeInTheDocument();
  expect(dialog.getByRole("button", { name: "Send invite" })).toBeDisabled();
  await userEvent.clear(dialog.getByLabelText("Email"));
  await userEvent.type(dialog.getByLabelText("Email"), "bob@acme.com");
  await userEvent.click(dialog.getByRole("button", { name: "Send invite" }));
  expect(await dialog.findByRole("alert")).toHaveTextContent("bob is already a member");
  expect(called(api, "POST", `${BASE}/invitations`)).toBeDefined();
  expect(dialog.getByLabelText("Email")).toHaveValue("bob@acme.com");
});

test("owners see only pending invitations and can resend and revoke them", async () => {
  const api = setup({
    "POST /api/v1/projects/{org}/{project}/members/invitations/{id}/resend": NO_CONTENT,
    "DELETE /api/v1/projects/{org}/{project}/members/invitations/{id}": NO_CONTENT,
  });
  renderAt("/p/acme/demo/members");
  const section = await screen.findByRole("region", { name: "Pending invitations" });
  expect(within(section).queryByText("old@example.com")).toBeNull();
  const row = within(section).getByRole("row", { name: /carol@example.com/ });
  expect(within(row).getByText("Editor")).toBeInTheDocument();
  await userEvent.click(within(row).getByRole("button", { name: "Resend" }));
  await waitFor(() => expect(called(api, "POST", `${BASE}/invitations/7/resend`)).toBeDefined());
  expect(await screen.findByText("Invitation sent again.")).toBeInTheDocument();
  await userEvent.click(within(row).getByRole("button", { name: "Revoke" }));
  expect(called(api, "DELETE", `${BASE}/invitations/7`)).toBeUndefined();
  await confirmWith("Revoke");
  await waitFor(() => expect(called(api, "DELETE", `${BASE}/invitations/7`)).toBeDefined());
  expect(await screen.findByText("Invitation revoked.")).toBeInTheDocument();
});

test("without pending invitations the section is absent", async () => {
  setup({
    "GET /api/v1/projects/{org}/{project}/members/invitations": () => ({ invitations: [revoked] }),
  });
  renderAt("/p/acme/demo/members");
  await screen.findByRole("row", { name: /Bob Builder/ });
  expect(screen.queryByRole("region", { name: "Pending invitations" })).toBeNull();
});

test("a non-owner sees the table read-only without invite or invitations", async () => {
  const api = setup({}, "reader");
  renderAt("/p/acme/demo/members");
  const bob = await screen.findByRole("row", { name: /Bob Builder/ });
  expect(within(bob).getByText("Editor")).toBeInTheDocument();
  expect(screen.queryByRole("combobox")).toBeNull();
  expect(screen.queryByRole("button", { name: "Invite" })).toBeNull();
  expect(screen.queryByRole("button", { name: /^Remove/ })).toBeNull();
  expect(called(api, "GET", `${BASE}/invitations`)).toBeUndefined();
});

test("the screen is available in Spanish", async () => {
  setup();
  renderAt("/p/acme/demo/members", "es");
  expect(await screen.findByRole("button", { name: "Invitar" })).toBeInTheDocument();
  expect(screen.getByRole("row", { name: /Gia Guest/ })).toHaveTextContent("Invitado");
});

test("project settings link to the members page instead of managing members", async () => {
  setup();
  renderAt("/p/acme/demo/settings");
  expect(await screen.findByRole("link", { name: "Open Members" })).toHaveAttribute(
    "href",
    "/p/acme/demo/members",
  );
  expect(screen.queryByRole("button", { name: "Grant role" })).toBeNull();
  expect(screen.getByLabelText("Repository URL")).toBeInTheDocument();
});
