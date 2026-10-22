import { expect, test } from "@playwright/test";

const app = process.env.APP_URL ?? "http://localhost:5173";

test("the seeded development person signs in and lands on onboarding", async ({ page }) => {
  await page.goto(`${app}/`);
  await page.fill("#username", "dev");
  await page.fill("#password", "${SPEC_STORE_DEV_PASSWORD}");
  await page.locator("input[type=submit], button[type=submit]").first().click();
  await expect(page).toHaveURL(new RegExp(`^${app}/(onboarding|o/)`));
  await expect(page.getByRole("link", { name: "dev" })).toBeVisible();
});
