import { mkdirSync, writeFileSync } from "node:fs";
import { join } from "node:path";
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

function writeTree(root: string) {
  mkdirSync(join(root, "docs"), { recursive: true });
  mkdirSync(join(root, "specs", "001-first"), { recursive: true });
  writeFileSync(join(root, "docs", "constitution.md"), "# Constitution\n\nVersion 1.0\n");
  writeFileSync(
    join(root, "specs", "001-first", "spec.md"),
    "# Spec 001\n\nStatus: approved\n\n- **FR-1** The system shall work.\n",
  );
  writeFileSync(
    join(root, "specs", "001-first", "tasks.md"),
    "# Tasks\n\n- [x] T01 (FR-1) First task — Done when: it works\n- [ ] T02 (FR-1) Second task — Done when: it works\n",
  );
}

test("the organization home lists project cards, creates a project with Get started, reads a folder, copies and archives", async ({
  page,
}, testInfo) => {
  const slug = `home-${Date.now().toString(36)}`;
  await signIn(page);
  await page.goto(`${app}/onboarding`);
  await page.fill("#slug", slug);
  await page.fill("#name", `Home ${slug}`);
  await page.getByRole("button", { name: "Create organization" }).click();
  await expect(page).toHaveURL(`${app}/o/${slug}`);

  await expect(page.getByRole("heading", { level: 1, name: "Projects" })).toBeVisible();
  await expect(page.getByText("You are not a member of any project yet")).toBeVisible();
  await page.screenshot({ path: `${evidence}/org-home-empty.png` });

  await page.getByRole("button", { name: "Create project" }).click();
  const dialog = page.getByRole("dialog", { name: "New project" });
  await dialog.getByLabel("Name").fill("Payments service");
  await expect(dialog.getByLabel("Slug")).toHaveValue("payments-service");
  await expect(dialog.getByText(`${slug}/payments-service`)).toBeVisible();
  await expect(dialog.getByRole("switch")).toBeDisabled();
  await page.screenshot({ path: `${evidence}/org-home-new-project.png` });
  await dialog.getByRole("button", { name: "Create project" }).click();

  await expect(page).toHaveURL(`${app}/p/${slug}/payments-service`);
  await expect(page.getByRole("heading", { name: "Get started" })).toBeVisible();
  await expect(page.getByText("1 of 4 done")).toBeVisible();
  await expect(page.getByText(/claude mcp add --transport http sdd-vault .*\/mcp/)).toBeVisible();
  await page.screenshot({ path: `${evidence}/org-home-get-started.png` });

  await page.goto(`${app}/o/${slug}`);
  const card = page.getByRole("link", { name: /Payments service/ });
  await expect(card).toBeVisible();
  await expect(card.getByText("Owner")).toBeVisible();
  await expect(card.getByText("No constitution yet")).toBeVisible();
  await page.screenshot({ path: `${evidence}/org-home-cards.png` });

  const tree = testInfo.outputPath("repo");
  writeTree(tree);
  await page.getByRole("button", { name: "Import SDD tree" }).click();
  const importDialog = page.getByRole("dialog", { name: "New project" });
  await expect(importDialog.getByRole("radio", { name: /Import SDD tree/ })).toBeChecked();
  await expect(importDialog.getByRole("button", { name: "Create and import" })).toBeDisabled();
  await importDialog.getByLabel("Name").fill("Imported");
  await importDialog.getByLabel("Choose folder").setInputFiles(tree);
  await expect(importDialog.getByText(/specs: 1, documents: 3/)).toBeVisible();
  await expect(importDialog.getByRole("button", { name: "Create and import" })).toBeEnabled();
  await page.screenshot({ path: `${evidence}/org-home-import.png` });
  await importDialog.getByRole("button", { name: "Create and import" }).click();
  await expect(page).toHaveURL(`${app}/p/${slug}/imported`, { timeout: 30_000 });
  await expect(page.getByRole("heading", { name: "Imported" })).toBeVisible();
  await expect(page.getByText(/001/).first()).toBeVisible();
  await page.screenshot({ path: `${evidence}/org-home-imported.png` });
  await page.goto(`${app}/o/${slug}`);

  await page.getByRole("button", { name: "Create project" }).click();
  const copyDialog = page.getByRole("dialog", { name: "New project" });
  await copyDialog.getByLabel("Name").fill("Payments copy");
  await copyDialog.getByRole("radio", { name: /Copy project/ }).click();
  await copyDialog.getByRole("combobox", { name: "Project to copy" }).click();
  await page.getByRole("option", { name: "Payments service" }).click();
  await page.screenshot({ path: `${evidence}/org-home-copy.png` });
  await copyDialog.getByRole("button", { name: "Copy project" }).click();
  await expect(page).toHaveURL(`${app}/p/${slug}/payments-copy`);
  await expect(page.getByRole("heading", { name: "Payments copy" })).toBeVisible();

  await page.goto(`${app}/p/${slug}/payments-service/settings`);
  await page.getByRole("button", { name: "Archive project" }).click();
  await page.getByRole("dialog").getByRole("button", { name: "Archive project" }).click();
  await expect(page.getByText("Project archived.")).toBeVisible();
  await page.goto(`${app}/p/${slug}/payments-service`);
  await expect(page.getByText(/archived and read-only/)).toBeVisible();
  await page.screenshot({ path: `${evidence}/org-home-archived-overview.png` });
  await page.goto(`${app}/o/${slug}`);
  await expect(page.getByRole("link", { name: /Payments service/ })).toBeHidden();
  await expect(page.getByRole("link", { name: /Payments copy/ })).toBeVisible();
  await page.getByRole("tab", { name: "Archived" }).click();
  await expect(
    page.getByRole("link", { name: /Payments service/ }).getByText("Archived"),
  ).toBeVisible();
  await page.screenshot({ path: `${evidence}/org-home-archived.png` });
});
