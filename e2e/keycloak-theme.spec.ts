import { expect, type Page, test } from "@playwright/test";

const keycloak = process.env.KEYCLOAK_URL ?? "http://localhost:58080";
const realm = `${keycloak}/realms/spec-store/protocol/openid-connect`;
const evidence = "specs/005-saas-foundation/evidence";
const pkce =
  "code_challenge=E9Melhoa2OwvFrEMTJguCHaoeK1t8URWbuGJSstw-cM&code_challenge_method=S256";
const client = "client_id=spec-store-app&response_type=code&scope=openid";
const redirect = `redirect_uri=${encodeURIComponent("http://localhost:5173/auth/callback")}`;

function authorize(path: string, language: string): string {
  return `${realm}/${path}?${client}&${redirect}&${pkce}&ui_locales=${language}`;
}

async function capture(page: Page, name: string, language: string) {
  await expect(page.locator("#kc-header-wrapper")).toContainText("spec-store");
  await page.screenshot({ path: `${evidence}/keycloak-${name}-${language}.png`, fullPage: true });
}

for (const language of ["en", "es"]) {
  test(`branded identity pages in ${language}`, async ({ page }) => {
    await page.goto(authorize("auth", language));
    await capture(page, "sign-in", language);

    await page.goto(authorize("registrations", language));
    await capture(page, "sign-up", language);

    const username = `theme-${language}-${Date.now()}`;
    await page.fill("#username", username);
    await page.fill("#email", `${username}@example.test`);
    await page.fill("#firstName", "Theme");
    await page.fill("#lastName", "Check");
    if (await page.locator("#password").count()) {
      await page.fill("#password", "${E2E_PASSWORD}");
      await page.fill("#password-confirm", "${E2E_PASSWORD}");
    }
    await page.locator("input[type=submit], button[type=submit]").first().click();
    await capture(page, "verify-email", language);

    const mailpit = process.env.MAILPIT_URL ?? "http://localhost:58025";
    let id = "";
    for (let attempt = 0; attempt < 20 && !id; attempt += 1) {
      const search = await page.request.get(`${mailpit}/api/v1/search`, {
        params: { query: `to:"${username}@example.test"` },
      });
      id = (await search.json()).messages?.[0]?.ID ?? "";
      if (!id) await page.waitForTimeout(250);
    }
    expect(id).not.toBe("");
    await page.goto(`${mailpit}/view/${id}.html`);
    await expect(page.locator("body")).toContainText("spec-store");
    await page.screenshot({ path: `${evidence}/keycloak-email-${language}.png`, fullPage: true });

    await page.context().clearCookies();
    await page.goto(authorize("auth", language));
    await page.locator("a[href*='reset-credentials']").first().click();
    await capture(page, "reset-password", language);

    await page.goto(
      `${realm}/auth?client_id=spec-store-app&response_type=code&redirect_uri=https%3A%2F%2Fevil.example&${pkce}&ui_locales=${language}`,
    );
    await capture(page, "error", language);
  });
}
