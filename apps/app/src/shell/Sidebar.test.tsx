import { screen, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { renderShell } from "~/test/shell";
import { parsePlace } from "./navigation";
import { Sidebar, type SidebarProps } from "./Sidebar";

const organization = { slug: "acme", name: "Acme Labs", role: "owner" };

function props(path: string, overrides: Partial<SidebarProps> = {}): SidebarProps {
  return {
    place: parsePlace(path),
    organization,
    back: { path: "/p/acme/demo/audit", label: "demo" },
    ...overrides,
  };
}

test("project mode lists the SDD and project screens and marks the current one", () => {
  renderShell(<Sidebar {...props("/p/acme/demo/specs/4/plan")} />, "/p/acme/demo/specs/4/plan");
  const nav = screen.getByRole("navigation", { name: "Main navigation" });
  expect(within(nav).getByText("SDD")).toBeInTheDocument();
  expect(within(nav).getByText("Project")).toBeInTheDocument();
  const labels = within(nav)
    .getAllByRole("link")
    .map((link) => link.textContent);
  expect(labels).toEqual([
    "Overview",
    "Active specs",
    "Specs",
    "Constitution",
    "Activity",
    "Members",
    "Audit",
    "Settings",
  ]);
  expect(within(nav).getByRole("link", { name: "Specs" })).toHaveAttribute("aria-current", "page");
  expect(within(nav).getByRole("link", { name: "Overview" })).not.toHaveAttribute("aria-current");
  expect(within(nav).getByRole("link", { name: "Audit" })).toHaveAttribute(
    "href",
    "/p/acme/demo/audit",
  );
});

test("an item navigates inside the app and reports the navigation", async () => {
  const onNavigate = vi.fn();
  renderShell(<Sidebar {...props("/p/acme/demo", { onNavigate })} />, "/p/acme/demo");
  await userEvent.click(screen.getByRole("link", { name: "Settings" }));
  expect(screen.getByTestId("location")).toHaveTextContent("/p/acme/demo/settings");
  expect(onNavigate).toHaveBeenCalledTimes(1);
});

test("account mode shows the way back and the account and organization screens", async () => {
  renderShell(<Sidebar {...props("/account/tokens")} />, "/account/tokens");
  const nav = screen.getByRole("navigation", { name: "Main navigation" });
  expect(within(nav).getByRole("link", { name: "Back to demo" })).toHaveAttribute(
    "href",
    "/p/acme/demo/audit",
  );
  expect(within(nav).getByText("Account")).toBeInTheDocument();
  expect(within(nav).getByText(/Organization/)).toHaveTextContent("Organization · Acme Labs");
  expect(
    within(nav)
      .getAllByRole("link")
      .map((link) => link.textContent),
  ).toEqual([
    "Back to demo",
    "Profile",
    "My tokens",
    "Preferences",
    "Security",
    "Billing & usage",
    "Audit",
  ]);
  expect(within(nav).getByRole("link", { name: "My tokens" })).toHaveAttribute(
    "aria-current",
    "page",
  );
  await userEvent.click(within(nav).getByRole("link", { name: "Back to demo" }));
  expect(screen.getByTestId("location")).toHaveTextContent("/p/acme/demo/audit");
});

test("organization security and billing screens use account mode", () => {
  renderShell(<Sidebar {...props("/o/acme/billing")} />, "/o/acme/billing");
  expect(screen.getByRole("link", { name: "Billing & usage" })).toHaveAttribute(
    "aria-current",
    "page",
  );
  expect(screen.getByRole("link", { name: "Profile" })).toBeInTheDocument();
});

test("the way back falls back to the organization name", () => {
  renderShell(
    <Sidebar {...props("/profile", { back: { path: "/o/acme", label: "Acme Labs" } })} />,
    "/profile",
  );
  expect(screen.getByRole("link", { name: "Back to Acme Labs" })).toHaveAttribute(
    "href",
    "/o/acme",
  );
});

test("an organization page without a project lists organization screens by role", () => {
  const { unmount } = renderShell(<Sidebar {...props("/o/acme")} />, "/o/acme");
  expect(screen.getAllByRole("link").map((link) => link.textContent)).toEqual([
    "Projects",
    "Members",
    "Invitations",
  ]);
  expect(screen.getByRole("link", { name: "Projects" })).toHaveAttribute("aria-current", "page");
  unmount();
  renderShell(
    <Sidebar {...props("/o/acme", { organization: { ...organization, role: "member" } })} />,
    "/o/acme",
  );
  expect(screen.getAllByRole("link").map((link) => link.textContent)).toEqual([
    "Projects",
    "Members",
  ]);
});

test("screens without an organization have no navigation items", () => {
  renderShell(<Sidebar {...props("/onboarding", { organization: undefined })} />, "/onboarding");
  expect(screen.queryAllByRole("link")).toEqual([]);
});
