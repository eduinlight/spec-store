import { expect, type Page, test } from "@playwright/test";

const app = process.env.APP_URL ?? "http://localhost:5173";
const keycloak =
  process.env.KEYCLOAK_URL ??
  process.env.KEYCLOAK_ISSUER?.replace(/\/realms\/.*$/, "") ??
  "http://localhost:58080";
const mailpit =
  process.env.MAILPIT_URL ?? `http://localhost:${process.env.MAILPIT_UI_PORT ?? "58025"}`;
const devPassword = process.env.SDD_VAULT_DEV_PASSWORD ?? "";
const evidence = "test-results/evidence";
const guestPassword = crypto.randomUUID();
const NAVIGATION = { timeout: 60_000 };

async function latestLink(page: Page, address: string, pattern: RegExp): Promise<string> {
  for (let attempt = 0; attempt < 120; attempt += 1) {
    const search = await page.request.get(`${mailpit}/api/v1/search`, {
      params: { query: `to:"${address}"` },
    });
    for (const found of (await search.json()).messages ?? []) {
      const message = await (
        await page.request.get(`${mailpit}/api/v1/message/${found.ID}`)
      ).json();
      for (const match of (message.HTML ?? "").matchAll(/href="([^"]+)"/g)) {
        const link = (match[1] ?? "").replaceAll("&amp;", "&");
        if (pattern.test(link)) return link;
      }
    }
    await page.waitForTimeout(500);
  }
  throw new Error(`no email with a matching link for ${address}`);
}

async function signInAsDev(page: Page) {
  expect(devPassword, "SDD_VAULT_DEV_PASSWORD comes from .env").not.toBe("");
  await page.goto(`${app}/`, NAVIGATION);
  await page.fill("#username", "dev");
  await page.fill("#password", devPassword);
  await page.locator("input[type=submit], button[type=submit]").first().click();
  await expect(page).toHaveURL(new RegExp(`^${app}/(onboarding|o/)`), NAVIGATION);
}

test("an owner invites an outside email, who joins as a guest and sees only that project", async ({
  page,
}) => {
  test.setTimeout(240_000);
  page.setDefaultNavigationTimeout(60_000);
  page.setDefaultTimeout(30_000);
  const suffix = Date.now().toString(36);
  const org = `mem-${suffix}`;
  const guestName = `guest-${suffix}`;
  const guestEmail = `${guestName}@example.test`;

  await signInAsDev(page);
  await page.goto(`${app}/onboarding`);
  await page.fill("#slug", org);
  await page.fill("#name", `Members ${suffix}`);
  await page.getByRole("button", { name: "Create organization" }).click();
  await expect(page).toHaveURL(`${app}/o/${org}`);

  await page.getByRole("button", { name: "Create project" }).click();
  const newProject = page.getByRole("dialog", { name: "New project" });
  await newProject.getByLabel("Name").fill("Payments service");
  await newProject.getByRole("button", { name: "Create project" }).click();
  await expect(page).toHaveURL(`${app}/p/${org}/payments-service`);

  await page.goto(`${app}/p/${org}/payments-service/members`);
  await expect(page.getByRole("heading", { level: 1, name: "Members" })).toBeVisible();
  await page.screenshot({ path: `${evidence}/members-owner.png` });

  await page.getByRole("button", { name: "Invite" }).click();
  const invite = page.getByRole("dialog", { name: "Invite to Payments service" });
  await invite.getByLabel("Email").fill(guestEmail);
  await invite.getByRole("combobox", { name: "Role" }).click();
  await page.getByRole("option", { name: "Editor" }).click();
  await expect(invite.getByText(/join as guests and see only this project/)).toBeVisible();
  await page.screenshot({ path: `${evidence}/members-invite-dialog.png` });
  await invite.getByRole("button", { name: "Send invite" }).click();
  await expect(page.getByText(`Invitation sent to ${guestEmail}`)).toBeVisible();

  const pending = page.getByRole("region", { name: "Pending invitations" });
  const pendingRow = pending.getByRole("row", { name: new RegExp(guestEmail) });
  await expect(pendingRow).toBeVisible();
  await expect(pendingRow.getByText("Editor")).toBeVisible();
  await page.screenshot({ path: `${evidence}/members-pending.png` });

  const accept = await latestLink(page, guestEmail, /\/invitations\//);

  await page.getByRole("button", { name: "User menu" }).click();
  await page.getByRole("menuitem", { name: "Sign out" }).click();
  await page.waitForURL(new RegExp(`^${keycloak}/realms/sdd-vault/protocol/openid-connect/auth`));

  await page.goto(accept);
  await expect(page).toHaveURL(
    new RegExp(`^${keycloak}/realms/sdd-vault/protocol/openid-connect/auth`),
  );
  await page.getByRole("link", { name: "Create an account" }).click();
  await page.fill("#username", guestName);
  await page.fill("#email", guestEmail);
  await page.fill("#firstName", "Gia");
  await page.fill("#lastName", "Guest");
  if (await page.locator("#password").count()) {
    await page.fill("#password", guestPassword);
    await page.fill("#password-confirm", guestPassword);
  }
  await page.locator("input[type=submit], button[type=submit]").first().click();
  const verify = await latestLink(page, guestEmail, /action-token|verify-email|login-actions/);
  await page.goto(verify);
  if (await page.locator("#password-new").count()) {
    await page.fill("#password-new", guestPassword);
    await page.fill("#password-confirm", guestPassword);
    await page.locator("input[type=submit], button[type=submit]").first().click();
  }
  const proceed = page.getByRole("link", {
    name: /back to application|click here to proceed|proceed/i,
  });
  if (await proceed.count()) await proceed.first().click();
  if (!page.url().includes("/invitations/")) await page.goto(accept);

  await expect(page.getByText("You are invited to join Payments service as Editor.")).toBeVisible();
  await page.screenshot({ path: `${evidence}/members-accept.png` });
  await page.getByRole("button", { name: "Accept invitation" }).click();
  await expect(page).toHaveURL(`${app}/p/${org}/payments-service`);

  await page.goto(`${app}/o/${org}`);
  await expect(page.getByRole("heading", { level: 1, name: "Projects" })).toBeVisible();
  await expect(page.getByRole("link", { name: /Payments service/ })).toHaveCount(1);
  await page.getByRole("button", { name: "Organization" }).click();
  await expect(page.getByRole("option", { name: new RegExp(`Members ${suffix}`) })).toContainText(
    "Guest",
  );
  await page.screenshot({ path: `${evidence}/members-guest-organization.png` });
  await page.keyboard.press("Escape");

  await page.goto(`${app}/p/${org}/payments-service/members`);
  const self = page.getByRole("row", { name: new RegExp(`@${guestName}`) });
  await expect(self.getByText("Guest", { exact: true })).toBeVisible();
  await expect(page.getByRole("button", { name: "Invite" })).toHaveCount(0);
  await page.screenshot({ path: `${evidence}/members-guest-view.png` });
});
