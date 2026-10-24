import { mkdirSync, writeFileSync } from "node:fs";
import { join } from "node:path";
import { expect, type Page, test, type WebSocket } from "@playwright/test";

const app = process.env.APP_URL ?? "http://localhost:5173";
const password = process.env.SDD_VAULT_DEV_PASSWORD ?? "";
const evidence = "test-results/evidence";
const NAVIGATION = { timeout: 30_000 };
const LIVE = { timeout: 5_000 };

async function signIn(page: Page) {
  expect(password, "SDD_VAULT_DEV_PASSWORD comes from .env").not.toBe("");
  await page.goto(`${app}/`);
  await page.fill("#username", "dev");
  await page.fill("#password", password);
  await page.locator("input[type=submit], button[type=submit]").first().click();
  await expect(page).toHaveURL(new RegExp(`^${app}/(onboarding|o/)`), NAVIGATION);
}

async function opensLive(page: Page, open: () => Promise<unknown>) {
  const subscribed = new Promise<void>((resolve) => {
    const watch = (socket: WebSocket) => {
      if (!socket.url().includes("/api/v1/live")) return;
      socket.on("framereceived", (frame) => {
        if (String(frame.payload).includes('"subscribed"')) {
          page.off("websocket", watch);
          resolve();
        }
      });
    };
    page.on("websocket", watch);
  });
  await open();
  await subscribed;
}

const apiHeaders = { "X-Sdd-Vault-Request": "1" };

test("a write through the API appears on the open Activity and Overview pages without reloading", async ({
  page,
}) => {
  const slug = `live-${Date.now().toString(36)}`;
  await signIn(page);
  await page.goto(`${app}/onboarding`);
  await page.fill("#slug", slug);
  await page.fill("#name", `Live ${slug}`);
  await page.getByRole("button", { name: "Create organization" }).click();
  await expect(page).toHaveURL(`${app}/o/${slug}`, NAVIGATION);

  await page.getByRole("button", { name: "Create project" }).click();
  const created = page.getByRole("dialog", { name: "New project" });
  await created.getByLabel("Name").fill("Pulse");
  await created.getByRole("button", { name: "Create project" }).click();
  await expect(page).toHaveURL(`${app}/p/${slug}/pulse`, NAVIGATION);
  await expect(page.getByRole("heading", { name: "Get started" })).toBeVisible(NAVIGATION);
  await expect(page.getByRole("region", { name: "Overview" })).toBeVisible();
  await page.screenshot({ path: `${evidence}/live-overview-empty.png` });

  await page.goto(`${app}/p/${slug}/pulse/active`);
  await expect(page.getByText("No active spec yet")).toBeVisible(NAVIGATION);
  await page.screenshot({ path: `${evidence}/live-active-empty.png` });

  await opensLive(page, () => page.goto(`${app}/p/${slug}/pulse/activity`));
  await expect(page.getByRole("heading", { level: 1, name: "Activity" })).toBeVisible(NAVIGATION);
  const feed = page.getByRole("list", { name: "Activity" });
  await expect(feed.getByText("created the project")).toBeVisible(NAVIGATION);
  await expect(page.getByText("Live", { exact: true })).toBeVisible(NAVIGATION);
  await page.evaluate(() => {
    (window as unknown as { liveMarker: string }).liveMarker = "same page";
  });
  await page.screenshot({ path: `${evidence}/live-activity-before.png` });

  const put = await page.request.put(`${app}/api/v1/projects/${slug}/pulse/repository-url`, {
    headers: apiHeaders,
    data: { url: "https://github.com/acme/pulse" },
  });
  expect(put.status()).toBe(204);
  await expect(feed.getByText("set the repository")).toBeVisible(LIVE);
  expect(await page.evaluate(() => (window as unknown as { liveMarker: string }).liveMarker)).toBe(
    "same page",
  );
  await page.screenshot({ path: `${evidence}/live-activity-after.png` });

  await page.getByRole("button", { name: "Security" }).click();
  await expect(page).toHaveURL(/filter=security/);
  await expect(feed.getByText("set the repository")).toBeVisible(NAVIGATION);
  await expect(feed.getByText("created the project")).toBeVisible();
  await page.getByRole("button", { name: "Tasks" }).click();
  await expect(page.getByText("Nothing matches this filter.")).toBeVisible(NAVIGATION);
  await page.screenshot({ path: `${evidence}/live-activity-filtered.png` });

  await page.setViewportSize({ width: 1280, height: 1700 });
  await opensLive(page, () => page.goto(`${app}/p/${slug}/pulse`));
  const recent = page.getByRole("list", { name: "Recent activity" });
  await expect(recent.getByText("set the repository")).toBeVisible(NAVIGATION);
  const cleared = await page.request.delete(`${app}/api/v1/projects/${slug}/pulse/repository-url`, {
    headers: apiHeaders,
  });
  expect(cleared.status()).toBe(204);
  await expect(recent.getByText("cleared the repository")).toBeVisible(LIVE);
  await page.screenshot({ path: `${evidence}/live-overview-live.png` });
});

function writeTree(root: string) {
  mkdirSync(join(root, "docs"), { recursive: true });
  mkdirSync(join(root, "specs", "001-refund-flow"), { recursive: true });
  writeFileSync(
    join(root, "docs", "constitution.md"),
    "# Principles\n\n1. Tests first.\n\nVersion 1.0\n",
  );
  writeFileSync(
    join(root, "specs", "001-refund-flow", "spec.md"),
    "# Refund flow\n\nStatus: approved\n\n## Summary\n\nRefund a captured payment.\n\n## Functional requirements\n\n- **FR-1** (US-1): THE SYSTEM SHALL create refunds.\n- **FR-2** (US-1): THE SYSTEM SHALL notify the customer.\n",
  );
  writeFileSync(
    join(root, "specs", "001-refund-flow", "tasks.md"),
    "# Tasks\n\n## Phase 1: Setup\n\n- [x] T01 (FR-1) Create the module — Done when: the crate builds · commit: abc1234\n- [ ] T02 [P] (FR-1, FR-2) Add the migration — Done when: the migration applies\n",
  );
}

test("Active specs and Overview show the next task and copy the agent prompt for an imported spec", async ({
  page,
  context,
}, testInfo) => {
  await context.grantPermissions(["clipboard-read", "clipboard-write"]);
  const slug = `active-${Date.now().toString(36)}`;
  await signIn(page);
  await page.goto(`${app}/onboarding`);
  await page.fill("#slug", slug);
  await page.fill("#name", `Active ${slug}`);
  await page.getByRole("button", { name: "Create organization" }).click();
  await expect(page).toHaveURL(`${app}/o/${slug}`, NAVIGATION);

  const tree = testInfo.outputPath("repo");
  writeTree(tree);
  await page.getByRole("button", { name: "Import SDD tree" }).click();
  const dialog = page.getByRole("dialog", { name: "New project" });
  await dialog.getByLabel("Name").fill("Payments");
  await dialog.getByLabel("Choose folder").setInputFiles(tree);
  await expect(dialog.getByText(/specs: 1/)).toBeVisible();
  await dialog.getByRole("button", { name: "Create and import" }).click();
  await expect(page).toHaveURL(`${app}/p/${slug}/payments`, { timeout: 60_000 });
  await page.setViewportSize({ width: 1280, height: 1500 });
  await expect(page.getByRole("link", { name: /Refund flow/ })).toBeVisible(NAVIGATION);
  await expect(page.getByText("0%", { exact: true })).toBeVisible();
  await page.screenshot({ path: `${evidence}/live-overview-data.png` });

  await page
    .getByRole("navigation", { name: "Main navigation" })
    .getByRole("link", { name: "Active specs" })
    .click();
  await expect(page).toHaveURL(`${app}/p/${slug}/payments/active`, NAVIGATION);
  const next = page.getByRole("region", { name: "Next task" });
  await expect(next.getByText("T02")).toBeVisible(NAVIGATION);
  await expect(next.getByText("Done when: the migration applies")).toBeVisible();
  await next.getByRole("button", { name: "Copy agent prompt" }).click();
  await expect(page.getByText("Agent prompt copied.")).toBeVisible();
  const prompt = await page.evaluate(() => navigator.clipboard.readText());
  expect(prompt).toContain(`${slug}/payments`);
  expect(prompt).toContain("T02");
  await page.screenshot({ path: `${evidence}/live-active-data.png` });

  await next.getByRole("button", { name: "All tasks" }).click();
  await expect(page).toHaveURL(`${app}/p/${slug}/payments/specs/1/tasks`, NAVIGATION);
});
