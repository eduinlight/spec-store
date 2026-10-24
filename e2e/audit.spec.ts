import { readFileSync } from "node:fs";
import { expect, type Page, test } from "@playwright/test";

const app = process.env.APP_URL ?? "http://localhost:5173";
const password = process.env.SDD_VAULT_DEV_PASSWORD ?? "";
const evidence = "test-results/evidence";
const NAVIGATION = { timeout: 30_000 };
const apiHeaders = { "X-Sdd-Vault-Request": "1" };

async function signIn(page: Page) {
  expect(password, "SDD_VAULT_DEV_PASSWORD comes from .env").not.toBe("");
  await page.goto(`${app}/`);
  await page.fill("#username", "dev");
  await page.fill("#password", password);
  await page.locator("input[type=submit], button[type=submit]").first().click();
  await expect(page).toHaveURL(new RegExp(`^${app}/(onboarding|o/)`), NAVIGATION);
}

test("the audit trail filters to the denied write, hides a column and exports the selection", async ({
  page,
}) => {
  const slug = `audit-${Date.now().toString(36)}`;
  await signIn(page);
  await page.goto(`${app}/onboarding`);
  await page.fill("#slug", slug);
  await page.fill("#name", `Audit ${slug}`);
  await page.getByRole("button", { name: "Create organization" }).click();
  await expect(page).toHaveURL(`${app}/o/${slug}`, NAVIGATION);

  await page.getByRole("button", { name: "Create project" }).click();
  const created = page.getByRole("dialog", { name: "New project" });
  await created.getByLabel("Name").fill("Ledger");
  await created.getByRole("button", { name: "Create project" }).click();
  await expect(page).toHaveURL(`${app}/p/${slug}/ledger`, NAVIGATION);

  const project = `${app}/api/v1/projects/${slug}/ledger`;
  const archived = await page.request.post(`${project}/archive`, { headers: apiHeaders });
  expect(archived.ok()).toBe(true);
  const refused = await page.request.put(`${project}/repository-url`, {
    headers: apiHeaders,
    data: { url: "https://github.com/acme/ledger" },
  });
  expect(refused.ok()).toBe(false);
  const restored = await page.request.post(`${project}/restore`, { headers: apiHeaders });
  expect(restored.ok()).toBe(true);

  await page.goto(`${app}/p/${slug}/ledger/audit`);
  await expect(page.getByRole("heading", { name: "Audit trail" })).toBeVisible(NAVIGATION);
  await expect(page.getByText("Immutable · append-only")).toBeVisible();
  const rows = page.getByRole("table", { name: "Audit entries" }).locator("tbody tr");
  await expect(rows.first()).toBeVisible(NAVIGATION);
  expect(await rows.count()).toBeGreaterThanOrEqual(4);
  await page.screenshot({ path: `${evidence}/audit-table.png` });

  await page.getByRole("button", { name: "Result" }).click();
  await page.getByRole("checkbox", { name: /Denied/ }).click();
  await page.keyboard.press("Escape");
  await expect(page).toHaveURL(/result=denied/, NAVIGATION);
  await expect(rows).toHaveCount(1, NAVIGATION);
  await expect(rows.first()).toContainText("Denied");
  await expect(page.getByText("0 of 1 row(s) selected.")).toBeVisible();
  await page.screenshot({ path: `${evidence}/audit-denied.png` });

  await page.getByRole("button", { name: "View" }).click();
  await page.getByRole("checkbox", { name: "Origin" }).uncheck();
  await expect(page.getByRole("columnheader", { name: "Origin" })).toHaveCount(0);
  await page.getByRole("button", { name: "View" }).click();

  await rows.first().getByRole("checkbox", { name: "Select row" }).check();
  await expect(page.getByText("1 of 1 row(s) selected.")).toBeVisible();
  await page.screenshot({ path: `${evidence}/audit-selected.png` });

  const downloading = page.waitForEvent("download", NAVIGATION);
  await page.getByRole("button", { name: "Export 1 selected" }).click();
  const download = await downloading;
  expect(download.suggestedFilename()).toMatch(/\.csv$/);
  const path = await download.path();
  const lines = readFileSync(path, "utf8").trim().split(/\r?\n/);
  expect(lines[0]).toBe("time,actor,client,action,target,result,detail,origin");
  expect(lines).toHaveLength(2);
  expect(lines[1]).toContain("denied");
  await expect(page.getByText("Export started.")).toBeVisible();

  await page.getByRole("button", { name: "Reset" }).click();
  await expect(page).not.toHaveURL(/result=/, NAVIGATION);
  await page.goto(`${app}/o/${slug}/audit`);
  await expect(page.getByRole("columnheader", { name: "Project" })).toBeVisible(NAVIGATION);
  await page.screenshot({ path: `${evidence}/audit-organization.png` });
});
