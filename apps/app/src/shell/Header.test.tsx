import { screen, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { renderShell } from "~/test/shell";
import { Header, type HeaderProps } from "./Header";

const organizations = [
  { slug: "acme", name: "Acme Labs", role: "owner" },
  { slug: "beta", name: "Beta Inc", role: "member" },
];

const projects = [
  { slug: "demo", name: "Demo", role: "owner" },
  { slug: "site", name: "Site", role: "reader" },
];

function props(overrides: Partial<HeaderProps> = {}): HeaderProps {
  return {
    organizations,
    projects,
    org: "acme",
    project: "demo",
    activeSpec: "005 saas-foundation",
    unread: 0,
    onOpenPalette: vi.fn(),
    onOpenShortcuts: vi.fn(),
    onOpenNotifications: vi.fn(),
    onOpenMenu: vi.fn(),
    ...overrides,
  };
}

test("the header shows the logo and the current organization and project with its role", () => {
  renderShell(<Header {...props()} />, "/p/acme/demo");
  const banner = screen.getByRole("banner");
  expect(within(banner).getByRole("img", { name: "SDD Vault" })).toHaveAttribute(
    "src",
    "/logo-horizontal-white.png",
  );
  expect(screen.getByRole("button", { name: "Organization" })).toHaveTextContent("Acme Labs");
  const project = screen.getByRole("button", { name: "Project" });
  expect(project).toHaveTextContent("demo");
  expect(project).toHaveTextContent("Owner");
});

test("the organization switcher searches, marks the current one and navigates", async () => {
  renderShell(<Header {...props()} />, "/o/acme/members");
  await userEvent.click(screen.getByRole("button", { name: "Organization" }));
  const options = screen.getAllByRole("option");
  expect(options.map((option) => option.textContent)).toEqual([
    expect.stringContaining("Acme Labs"),
    expect.stringContaining("Beta Inc"),
  ]);
  expect(options[0]).toHaveAttribute("aria-selected", "true");
  await userEvent.type(screen.getByPlaceholderText("Search organization…"), "bet");
  expect(screen.getAllByRole("option")).toHaveLength(1);
  await userEvent.click(screen.getByRole("option", { name: /Beta Inc/ }));
  expect(screen.getByTestId("location")).toHaveTextContent("/o/beta/members");
});

test("guest organizations carry a Guest tag in the switcher", async () => {
  renderShell(
    <Header
      {...props({
        organizations: [
          { slug: "acme", name: "Acme Labs", role: "owner" },
          { slug: "ext", name: "Outside Co", role: null, guest: true },
        ],
      })}
    />,
    "/o/acme",
  );
  await userEvent.click(screen.getByRole("button", { name: "Organization" }));
  expect(screen.getByRole("option", { name: /Outside Co/ })).toHaveTextContent("Guest");
  expect(screen.getByRole("option", { name: /Acme Labs/ })).not.toHaveTextContent("Guest");
});

test("the organization switcher offers to create an organization", async () => {
  renderShell(<Header {...props()} />, "/o/acme");
  await userEvent.click(screen.getByRole("button", { name: "Organization" }));
  await userEvent.click(screen.getByRole("button", { name: "Create organization" }));
  expect(screen.getByTestId("location")).toHaveTextContent("/onboarding");
});

test("the project switcher lists the projects with role and active spec and keeps the screen", async () => {
  renderShell(<Header {...props()} />, "/p/acme/demo/audit");
  await userEvent.click(screen.getByRole("button", { name: "Project" }));
  expect(screen.getByText("Projects in Acme Labs")).toBeInTheDocument();
  const current = screen.getByRole("option", { name: /demo/ });
  expect(current).toHaveAttribute("aria-selected", "true");
  expect(current).toHaveTextContent("005 saas-foundation");
  expect(current).toHaveTextContent("Owner");
  expect(screen.getByRole("option", { name: /site/ })).toHaveTextContent("Reader");
  await userEvent.click(screen.getByRole("option", { name: /site/ }));
  expect(screen.getByTestId("location")).toHaveTextContent("/p/acme/site/audit");
});

test("the project switcher offers all projects and a new project", async () => {
  renderShell(<Header {...props()} />, "/p/acme/demo");
  await userEvent.click(screen.getByRole("button", { name: "Project" }));
  await userEvent.click(screen.getByRole("button", { name: "All projects" }));
  expect(screen.getByTestId("location")).toHaveTextContent("/o/acme");
  await userEvent.click(screen.getByRole("button", { name: "Project" }));
  await userEvent.click(screen.getByRole("button", { name: "New project" }));
  expect(screen.getByTestId("location")).toHaveTextContent("/o/acme/projects/new");
});

test("without a project the switcher asks to select one", () => {
  renderShell(<Header {...props({ project: undefined })} />, "/o/acme");
  expect(screen.getByRole("button", { name: "Project" })).toHaveTextContent("Select a project");
});

test("without organizations only the logo and the triggers remain", () => {
  renderShell(<Header {...props({ organizations: [], org: undefined, project: undefined })} />);
  expect(screen.queryByRole("button", { name: "Organization" })).not.toBeInTheDocument();
  expect(screen.queryByRole("button", { name: "Project" })).not.toBeInTheDocument();
});

test("the triggers open the palette, the shortcuts, the notifications and the menu", async () => {
  const handlers = props();
  renderShell(<Header {...handlers} />);
  const trigger = screen.getByRole("button", { name: "Search or jump to…" });
  expect(trigger).toHaveTextContent("K");
  await userEvent.click(trigger);
  await userEvent.click(screen.getByRole("button", { name: "Keyboard shortcuts" }));
  await userEvent.click(screen.getByRole("button", { name: "Notifications" }));
  await userEvent.click(screen.getByRole("button", { name: "Open menu" }));
  expect(handlers.onOpenPalette).toHaveBeenCalledTimes(1);
  expect(handlers.onOpenShortcuts).toHaveBeenCalledTimes(1);
  expect(handlers.onOpenNotifications).toHaveBeenCalledTimes(1);
  expect(handlers.onOpenMenu).toHaveBeenCalledTimes(1);
});

test("the notifications button shows a badge only when something is unread", () => {
  const { unmount } = renderShell(<Header {...props({ unread: 0 })} />);
  expect(screen.queryByText("3")).not.toBeInTheDocument();
  unmount();
  renderShell(<Header {...props({ unread: 3 })} />);
  expect(screen.getByText("3")).toBeInTheDocument();
  expect(screen.getByRole("button", { name: "Notifications, 3 unread" })).toBeInTheDocument();
});

test("picking an organization reports the change before navigating", async () => {
  const handlers = props({ onOrganizationChange: vi.fn() });
  renderShell(<Header {...handlers} />, "/profile");
  await userEvent.click(screen.getByRole("button", { name: "Organization" }));
  await userEvent.click(screen.getByRole("option", { name: /Beta Inc/ }));
  expect(handlers.onOrganizationChange).toHaveBeenCalledWith("beta");
  expect(screen.getByTestId("location")).toHaveTextContent("/profile");
});
