import { expect, type Page, test } from "@playwright/test";

const app = process.env.APP_URL ?? "http://localhost:5173";
const password = process.env.SDD_VAULT_DEV_PASSWORD ?? "";
const evidence = "test-results/evidence";

async function signIn(page: Page) {
  expect(password, "SDD_VAULT_DEV_PASSWORD comes from .env").not.toBe("");
  await page.goto(`${app}/`);
  await page.fill("#username", "dev");
  await page.fill("#password", password);
  await page.locator("input[type=submit], button[type=submit]").first().click();
  await expect(page).toHaveURL(new RegExp(`^${app}/(onboarding|o/)`));
}

async function createWorkspace(page: Page, slug: string) {
  await page.goto(`${app}/onboarding`);
  await page.fill("#slug", slug);
  await page.fill("#name", `Shell ${slug}`);
  await page.getByRole("button", { name: "Create organization" }).click();
  await expect(page).toHaveURL(`${app}/o/${slug}`);
  await page.goto(`${app}/o/${slug}/projects/new`);
  await page.fill("#slug", "payments");
  await page.fill("#name", "Payments");
  await page.getByRole("button", { name: "Create project" }).click();
  await expect(page).toHaveURL(`${app}/p/${slug}/payments/settings`);
}

test("the shell wraps the app with the header, sidebar, palette, shortcuts and user menu", async ({
  page,
}) => {
  const slug = `shell-${Date.now().toString(36)}`;
  await signIn(page);
  await createWorkspace(page, slug);

  const header = page.getByRole("banner");
  await expect(header).toBeVisible();
  await expect(header.locator('img[alt="SDD Vault"]')).toBeVisible();
  await expect(page.getByRole("button", { name: "Organization" })).toContainText(`Shell ${slug}`);
  await expect(page.getByRole("button", { name: "Project" })).toContainText("payments");
  expect(await header.evaluate((element) => getComputedStyle(element).backgroundColor)).not.toBe(
    "rgba(0, 0, 0, 0)",
  );
  expect(await header.evaluate((element) => element.getBoundingClientRect().height)).toBe(60);
  const nav = page.getByRole("navigation", { name: "Main navigation" });
  await expect(nav.getByRole("link", { name: "Settings" })).toHaveAttribute("aria-current", "page");
  await page.screenshot({ path: `${evidence}/shell-project-settings.png` });

  await page.keyboard.press("g");
  await page.keyboard.press("a");
  await expect(page).toHaveURL(`${app}/p/${slug}/payments/activity`);
  await expect(page.getByRole("heading", { level: 1, name: "Activity" })).toBeVisible();
  await page.screenshot({ path: `${evidence}/shell-placeholder.png` });

  await page.keyboard.press("Control+k");
  const palette = page.getByRole("dialog", { name: "Command palette" });
  await expect(palette).toBeVisible();
  await expect(palette.getByRole("combobox")).toBeFocused();
  await page.screenshot({ path: `${evidence}/shell-palette.png` });
  await palette.getByRole("combobox").fill("constitution");
  await page.keyboard.press("Enter");
  await expect(page).toHaveURL(`${app}/p/${slug}/payments/constitution`);

  await page.keyboard.press("?");
  await expect(page.getByRole("dialog", { name: "Keyboard shortcuts" })).toBeVisible();
  await page.screenshot({ path: `${evidence}/shell-shortcuts.png` });
  await page.keyboard.press("Escape");
  await page.keyboard.press("n");
  await expect(page.getByText("Nothing waiting for you.")).toBeVisible();
  await page.screenshot({ path: `${evidence}/shell-notifications.png` });
  await page.keyboard.press("Escape");

  await page.getByRole("button", { name: "Project" }).click();
  await expect(page.getByRole("option", { name: /payments/ })).toHaveAttribute(
    "aria-selected",
    "true",
  );
  await page.screenshot({ path: `${evidence}/shell-project-switcher.png` });
  await page.keyboard.press("Escape");

  await page.getByRole("button", { name: "User menu" }).click();
  await page.screenshot({ path: `${evidence}/shell-user-menu.png` });
  await page.getByRole("menuitem", { name: "My tokens" }).click();
  await expect(page).toHaveURL(`${app}/account/tokens`);
  await expect(nav.getByText("Account", { exact: true })).toBeVisible();
  await page.screenshot({ path: `${evidence}/shell-account-mode.png` });
  await nav.getByRole("link", { name: "Back to payments" }).click();
  await expect(page).toHaveURL(`${app}/p/${slug}/payments/constitution`);
});

test("on a phone the sidebar opens in a drawer and nothing scrolls sideways", async ({ page }) => {
  const slug = `phone-${Date.now().toString(36)}`;
  await signIn(page);
  await createWorkspace(page, slug);
  await page.setViewportSize({ width: 390, height: 844 });
  await page.goto(`${app}/p/${slug}/payments/activity`);
  await expect(page.getByRole("heading", { level: 1, name: "Activity" })).toBeVisible();
  const overflow = await page.evaluate(
    () => document.documentElement.scrollWidth - document.documentElement.clientWidth,
  );
  expect(overflow).toBeLessThanOrEqual(0);
  await expect(page.getByRole("navigation", { name: "Main navigation" })).toBeHidden();
  await page.screenshot({ path: `${evidence}/shell-phone.png` });
  await page.getByRole("button", { name: "Open menu" }).click();
  const drawer = page.getByRole("dialog");
  await expect(drawer.getByRole("link", { name: "Settings" })).toBeVisible();
  await page.screenshot({ path: `${evidence}/shell-phone-drawer.png` });
  await drawer.getByRole("link", { name: "Settings" }).click();
  await expect(page).toHaveURL(`${app}/p/${slug}/payments/settings`);
  await expect(page.getByRole("dialog")).toBeHidden();
});
