import { expect, type Page, test } from "@playwright/test";

const keycloak =
  process.env.KEYCLOAK_URL ??
  process.env.KEYCLOAK_ISSUER?.replace(/\/realms\/.*$/, "") ??
  "http://localhost:58080";
const app = process.env.APP_URL ?? "http://localhost:5173";
const landing = process.env.E2E_LANDING_URL ?? process.env.LANDING_URL ?? "http://localhost:5174";
const signIn = `${keycloak}/realms/sdd-vault/protocol/openid-connect/auth?client_id=sdd-vault-app&response_type=code&scope=openid&redirect_uri=${encodeURIComponent(`${app}/auth/callback`)}&code_challenge=E9Melhoa2OwvFrEMTJguCHaoeK1t8URWbuGJSstw-cM&code_challenge_method=S256`;

async function expectBrand(page: Page) {
  await expect(page.locator("body")).toContainText("SDD Vault");
  await expect(page.locator("body")).not.toContainText(/spec[- ]?store/i);
  for (const size of ["16x16", "32x32"]) {
    await expect(page.locator(`link[rel="icon"][sizes="${size}"]`)).toHaveCount(1);
  }
  await expect(page.locator('link[rel="apple-touch-icon"]')).toHaveCount(1);
  await expect(page.locator('meta[name="theme-color"]')).toHaveAttribute("content", "#1F4FE0");
}

test("the landing page carries the SDD Vault brand", async ({ page }) => {
  await page.goto(landing);
  await expectBrand(page);
  await expect(page.locator('img[alt="SDD Vault"]').first()).toBeVisible();
});

test("the identity provider carries the SDD Vault brand", async ({ page }) => {
  await page.goto(signIn);
  await expect(page.locator('.sdd-logo img[alt="SDD Vault"]')).toBeVisible();
  await expect(page.locator("body")).toContainText("SDD Vault");
  await expect(page.locator("body")).not.toContainText(/spec[- ]?store/i);
  await expect(page.locator('link[rel="icon"]')).toHaveCount(1);
  await expect(page.locator('meta[name="theme-color"]')).toHaveAttribute("content", "#1F4FE0");
});

test("the app document links the SDD Vault icons", async ({ request }) => {
  const html = await (await request.get(`${app}/index.html`)).text();
  expect(html).toContain("<title>SDD Vault</title>");
  expect(html).toMatch(/rel="icon"[^>]*sizes="32x32"/);
  expect(html).toMatch(/rel="apple-touch-icon"/);
  expect(html).toContain('content="#1F4FE0"');
});
