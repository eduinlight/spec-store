import { expect, type Page, test } from "@playwright/test";

const app = process.env.APP_URL ?? "http://localhost:5173";
const keycloak =
  process.env.KEYCLOAK_URL ??
  process.env.KEYCLOAK_ISSUER?.replace(/\/realms\/.*$/, "") ??
  "http://localhost:58080";
const mailpit =
  process.env.MAILPIT_URL ?? `http://localhost:${process.env.MAILPIT_UI_PORT ?? "58025"}`;
const evidence = "test-results/evidence";
const password = crypto.randomUUID();

async function latestLink(page: Page, address: string): Promise<string> {
  for (let attempt = 0; attempt < 40; attempt += 1) {
    const search = await page.request.get(`${mailpit}/api/v1/search`, {
      params: { query: `to:"${address}"` },
    });
    const id = (await search.json()).messages?.[0]?.ID;
    if (id) {
      const message = await (await page.request.get(`${mailpit}/api/v1/message/${id}`)).json();
      const match = /href="([^"]+)"/.exec(message.HTML ?? "");
      if (match?.[1]) return match[1].replaceAll("&amp;", "&");
    }
    await page.waitForTimeout(250);
  }
  throw new Error(`no email for ${address}`);
}

test("a new person signs up, verifies the email, lands in the app and signs out", async ({
  page,
}) => {
  const username = `e2e-${Date.now()}`;
  const email = `${username}@example.test`;
  await page.goto(`${app}/profile`);
  await expect(page).toHaveURL(
    new RegExp(`^${keycloak}/realms/sdd-vault/protocol/openid-connect/auth`),
  );
  await page.getByRole("link", { name: "Create an account" }).click();
  await page.fill("#username", username);
  await page.fill("#email", email);
  await page.fill("#firstName", "End");
  await page.fill("#lastName", "ToEnd");
  if (await page.locator("#password").count()) {
    await page.fill("#password", password);
    await page.fill("#password-confirm", password);
  }
  await page.locator("input[type=submit], button[type=submit]").first().click();
  await page.goto(await latestLink(page, email));
  if (await page.locator("#password-new").count()) {
    await page.fill("#password-new", password);
    await page.fill("#password-confirm", password);
    await page.locator("input[type=submit], button[type=submit]").first().click();
  }
  const proceed = page.getByRole("link", {
    name: /back to application|click here to proceed|proceed/i,
  });
  if (await proceed.count()) await proceed.first().click();
  await expect(page).toHaveURL(`${app}/profile`);
  await expect(page.getByRole("heading", { name: "Your profile" })).toBeVisible();
  await expect(page.getByText(email)).toBeVisible();

  await page.getByRole("button", { name: "Sign out" }).click();
  await page.waitForURL(new RegExp(`^${keycloak}/realms/sdd-vault/protocol/openid-connect/auth`));
  await page.goto(`${app}/profile`);
  await expect(page).toHaveURL(
    new RegExp(`^${keycloak}/realms/sdd-vault/protocol/openid-connect/auth`),
  );
  await expect(page.locator("#username")).toBeVisible();
});

test("the sign-in page follows the Spanish language choice", async ({ page, context }) => {
  await context.addCookies([{ name: "sdd_vault_lang", value: "es", url: app }]);
  await page.goto(`${app}/`);
  await expect(page).toHaveURL(/ui_locales=es|kc_locale=es|realms\/sdd-vault/);
  await expect(page.getByRole("heading", { name: /Inicia sesión/ })).toBeVisible();
  await page.screenshot({ path: `${evidence}/app-sign-in-es.png`, fullPage: true });
});
