import { screen, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { renderShell } from "~/test/shell";
import { UserMenu, type UserMenuProps } from "./UserMenu";

function props(overrides: Partial<UserMenuProps> = {}): UserMenuProps {
  return {
    person: { handle: "alice", name: "Alice Smith", email: "alice@example.com", isOperator: false },
    organization: { slug: "acme", name: "Acme Labs", role: "owner" },
    language: "en",
    onLanguage: vi.fn(),
    onSignOut: vi.fn(),
    ...overrides,
  };
}

test("the menu is pinned with the avatar, name and handle and opens on demand", async () => {
  renderShell(<UserMenu {...props()} />);
  const trigger = screen.getByRole("button", { name: "User menu" });
  expect(trigger).toHaveTextContent("Alice Smith");
  expect(trigger).toHaveTextContent("@alice");
  expect(trigger).toHaveAttribute("aria-expanded", "false");
  expect(screen.queryByRole("menu")).not.toBeInTheDocument();
  await userEvent.click(trigger);
  expect(trigger).toHaveAttribute("aria-expanded", "true");
  expect(screen.getByRole("menu")).toBeInTheDocument();
});

test("it lists account items, organization items, the language choice and sign out", async () => {
  renderShell(<UserMenu {...props()} />);
  await userEvent.click(screen.getByRole("button", { name: "User menu" }));
  const menu = screen.getByRole("menu");
  expect(within(menu).getByText("alice@example.com")).toBeInTheDocument();
  expect(
    within(menu)
      .getAllByRole("menuitem")
      .map((item) => item.textContent),
  ).toEqual([
    "Profile",
    "My tokens",
    "Preferences",
    "Security",
    "Billing & usage",
    "Audit",
    "Sign out",
  ]);
  expect(within(menu).getByText("Acme Labs")).toBeInTheDocument();
  const languages = within(menu).getAllByRole("menuitemradio");
  expect(languages.map((item) => item.textContent)).toEqual(["English", "Español"]);
  expect(languages[0]).toHaveAttribute("aria-checked", "true");
  expect(languages[1]).toHaveAttribute("aria-checked", "false");
});

test("choosing an account item navigates and closes the menu", async () => {
  renderShell(<UserMenu {...props()} />, "/p/acme/demo");
  await userEvent.click(screen.getByRole("button", { name: "User menu" }));
  await userEvent.click(screen.getByRole("menuitem", { name: "My tokens" }));
  expect(screen.getByTestId("location")).toHaveTextContent("/account/tokens");
  expect(screen.queryByRole("menu")).not.toBeInTheDocument();
  await userEvent.click(screen.getByRole("button", { name: "User menu" }));
  await userEvent.click(screen.getByRole("menuitem", { name: "Billing & usage" }));
  expect(screen.getByTestId("location")).toHaveTextContent("/o/acme/billing");
});

test("without an organization only the account items are listed", async () => {
  renderShell(<UserMenu {...props({ organization: undefined })} />);
  await userEvent.click(screen.getByRole("button", { name: "User menu" }));
  expect(screen.getAllByRole("menuitem").map((item) => item.textContent)).toEqual([
    "Profile",
    "My tokens",
    "Preferences",
    "Sign out",
  ]);
});

test("the language and sign out entries call their handlers", async () => {
  const handlers = props();
  renderShell(<UserMenu {...handlers} />);
  await userEvent.click(screen.getByRole("button", { name: "User menu" }));
  await userEvent.click(screen.getByRole("menuitemradio", { name: "Español" }));
  expect(handlers.onLanguage).toHaveBeenCalledWith("es");
  await userEvent.click(screen.getByRole("button", { name: "User menu" }));
  await userEvent.click(screen.getByRole("menuitem", { name: "Sign out" }));
  expect(handlers.onSignOut).toHaveBeenCalledTimes(1);
});

test("Escape and an outside click close the menu and Escape returns the focus", async () => {
  renderShell(
    <>
      <UserMenu {...props()} />
      <button type="button">outside</button>
    </>,
  );
  const trigger = screen.getByRole("button", { name: "User menu" });
  await userEvent.click(trigger);
  await userEvent.keyboard("{Escape}");
  expect(screen.queryByRole("menu")).not.toBeInTheDocument();
  expect(trigger).toHaveFocus();
  await userEvent.click(trigger);
  await userEvent.click(screen.getByRole("button", { name: "outside" }));
  expect(screen.queryByRole("menu")).not.toBeInTheDocument();
});

test("arrow keys move through the items and the first item takes the focus on open", async () => {
  renderShell(<UserMenu {...props()} />);
  await userEvent.click(screen.getByRole("button", { name: "User menu" }));
  expect(screen.getByRole("menuitem", { name: "Profile" })).toHaveFocus();
  await userEvent.keyboard("{ArrowDown}");
  expect(screen.getByRole("menuitem", { name: "My tokens" })).toHaveFocus();
  await userEvent.keyboard("{ArrowUp}{ArrowUp}");
  expect(screen.getByRole("menuitem", { name: "Sign out" })).toHaveFocus();
});

test("an operator is labelled in the menu header", async () => {
  renderShell(
    <UserMenu
      {...props({
        person: { handle: "root", name: "Root", email: "root@example.com", isOperator: true },
      })}
    />,
  );
  await userEvent.click(screen.getByRole("button", { name: "User menu" }));
  expect(within(screen.getByRole("menu")).getByText("Operator")).toBeInTheDocument();
});
