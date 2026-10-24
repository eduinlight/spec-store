import { expect, type Page, test } from "@playwright/test";

const keycloak =
  process.env.KEYCLOAK_URL ??
  process.env.KEYCLOAK_ISSUER?.replace(/\/realms\/.*$/, "") ??
  "http://localhost:58080";
const realm = `${keycloak}/realms/sdd-vault/protocol/openid-connect`;
const landing = process.env.E2E_LANDING_URL ?? process.env.LANDING_URL ?? "http://localhost:5174";
const evidence = "test-results/evidence";
const password = crypto.randomUUID();
const pkce =
  "code_challenge=E9Melhoa2OwvFrEMTJguCHaoeK1t8URWbuGJSstw-cM&code_challenge_method=S256";
const client = "client_id=sdd-vault-app&response_type=code&scope=openid";
const redirect = `redirect_uri=${encodeURIComponent(`${process.env.APP_URL ?? "http://localhost:5173"}/auth/callback`)}`;

function authorize(path: string, language: string): string {
  return `${realm}/${path}?${client}&${redirect}&${pkce}&ui_locales=${language}`;
}

const texts = {
  en: {
    signInTitle: "Sign in",
    signInSubtitle: "Use your username or email and your password.",
    newTo: "New to SDD Vault?",
    create: "Create an account",
    registerTitle: "Create your account",
    registerSubtitle: "Free for up to 3 projects. A personal organization is created for you.",
    haveAccount: "Already have an account?",
    signInLink: "Sign in",
    terms: "By continuing you agree to the Terms and Privacy Policy.",
    eyebrow: "Prompts are suggestions",
    heading: "SDD Vault rules are enforced.",
    error: "Only project owners can write the constitution.",
    trustOne: "SSO and SCIM · Scoped agent tokens · Immutable audit trail",
    trustTwo: "EU and US data regions",
  },
  es: {
    signInTitle: "Inicia sesión",
    signInSubtitle: "Usa tu nombre de usuario o correo y tu contraseña.",
    newTo: "¿Nuevo en SDD Vault?",
    create: "Crea una cuenta",
    registerTitle: "Crea tu cuenta",
    registerSubtitle: "Gratis hasta 3 proyectos. Se crea una organización personal para ti.",
    haveAccount: "¿Ya tienes una cuenta?",
    signInLink: "Inicia sesión",
    terms: "Al continuar aceptas los Términos y la Política de privacidad.",
    eyebrow: "Los prompts son sugerencias",
    heading: "Las reglas de SDD Vault se aplican.",
    error: "Solo los propietarios del proyecto pueden escribir la constitución.",
    trustOne: "SSO y SCIM · Tokens de agente con alcance · Auditoría inmutable",
    trustTwo: "Regiones de datos en la UE y EE. UU.",
  },
} as const;

async function expectPanel(page: Page, language: "en" | "es") {
  const t = texts[language];
  const panel = page.locator(".sdd-panel");
  await expect(panel).toBeVisible();
  await expect(panel).toContainText(t.eyebrow, { ignoreCase: true });
  await expect(panel.getByRole("heading", { name: t.heading })).toBeVisible();
  await expect(panel).toContainText('put_constitution(project: "payments", base_revision: 7)');
  await expect(panel).toContainText("403 authorization_error");
  await expect(panel).toContainText(t.error);
  await expect(panel).toContainText(t.trustOne);
  await expect(panel).toContainText(t.trustTwo);
}

async function expectNoOverflow(page: Page) {
  const overflow = await page.evaluate(
    () => document.documentElement.scrollWidth - document.documentElement.clientWidth,
  );
  expect(overflow).toBeLessThanOrEqual(0);
}

async function capture(
  page: Page,
  name: string,
  language: string,
  textsLanguage: "en" | "es" = language as "en" | "es",
) {
  await expect(page.locator('.sdd-logo img[alt="SDD Vault"]')).toBeVisible();
  await expectPanel(page, textsLanguage);
  await page.screenshot({ path: `${evidence}/keycloak-${name}-${language}.png`, fullPage: true });
}

for (const language of ["en", "es"]) {
  test(`branded identity pages in ${language}`, async ({ page }) => {
    const t = texts[language as "en" | "es"];
    await page.goto(authorize("auth", language));
    await expect(page.locator(".sdd-logo")).toHaveAttribute("href", landing);
    await expect(page.getByRole("heading", { level: 1, name: t.signInTitle })).toBeVisible();
    await expect(page.locator(".sdd-subtitle")).toHaveText(t.signInSubtitle);
    await expect(page.locator("#sdd-switch")).toHaveText(`${t.newTo} ${t.create}`);
    await expect(page.locator("#sdd-switch a")).toHaveAttribute(
      "href",
      /login-actions\/registration/,
    );
    await expect(page.locator("#sdd-terms")).toHaveText(t.terms);
    await capture(page, "sign-in", language);

    await page.goto(authorize("registrations", language));
    await expect(page.getByRole("heading", { level: 1, name: t.registerTitle })).toBeVisible();
    await expect(page.locator(".sdd-subtitle")).toHaveText(t.registerSubtitle);
    await expect(page.locator("#sdd-switch")).toHaveText(`${t.haveAccount} ${t.signInLink}`);
    await expect(page.locator("#sdd-switch a")).toHaveAttribute(
      "href",
      /login-actions\/authenticate/,
    );
    await expect(page.locator("#sdd-terms")).toHaveText(t.terms);
    await capture(page, "sign-up", language);

    const username = `theme-${language}-${Date.now()}`;
    await page.fill("#username", username);
    await page.fill("#email", `${username}@example.test`);
    await page.fill("#firstName", "Theme");
    await page.fill("#lastName", "Check");
    if (await page.locator("#password").count()) {
      await page.fill("#password", password);
      await page.fill("#password-confirm", password);
    }
    await page.locator("input[type=submit], button[type=submit]").first().click();
    await capture(page, "verify-email", language);

    const mailpit =
      process.env.MAILPIT_URL ?? `http://localhost:${process.env.MAILPIT_UI_PORT ?? "58025"}`;
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
    await expect(page.locator("body")).toContainText("SDD Vault");
    await page.screenshot({ path: `${evidence}/keycloak-email-${language}.png`, fullPage: true });

    await page.context().clearCookies();
    await page.goto(authorize("auth", language));
    await page.locator("a[href*='reset-credentials']").first().click();
    await capture(page, "reset-password", language);

    await page.goto(
      `${realm}/auth?client_id=sdd-vault-app&response_type=code&redirect_uri=https%3A%2F%2Fevil.example&${pkce}&ui_locales=${language}`,
    );
    await capture(page, "error", language, "en");
  });
}

for (const language of ["en", "es"] as const) {
  test(`narrow screens stack the panel below the form in ${language}`, async ({ page }) => {
    await page.setViewportSize({ width: 390, height: 844 });
    for (const path of ["auth", "registrations"]) {
      await page.goto(authorize(path, language));
      await expectPanel(page, language);
      await expectNoOverflow(page);
      const form = await page.locator(".sdd-main").boundingBox();
      const panel = await page.locator(".sdd-panel").boundingBox();
      expect(panel?.y ?? 0).toBeGreaterThanOrEqual((form?.y ?? 0) + (form?.height ?? 0) - 1);
    }
    await page.screenshot({
      path: `${evidence}/keycloak-sign-up-narrow-${language}.png`,
      fullPage: true,
    });
  });
}
