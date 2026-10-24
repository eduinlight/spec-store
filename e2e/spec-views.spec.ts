import { mkdirSync, writeFileSync } from "node:fs";
import { join } from "node:path";
import { expect, type Page, test } from "@playwright/test";

const app = process.env.APP_URL ?? "http://localhost:5173";
const password = process.env.SDD_VAULT_DEV_PASSWORD ?? "";
const evidence = "test-results/evidence";
const NAVIGATION = { timeout: 30_000 };

async function signIn(page: Page) {
  expect(password, "SDD_VAULT_DEV_PASSWORD comes from .env").not.toBe("");
  await page.goto(`${app}/`);
  await page.fill("#username", "dev");
  await page.fill("#password", password);
  await page.locator("input[type=submit], button[type=submit]").first().click();
  await expect(page).toHaveURL(new RegExp(`^${app}/(onboarding|o/)`), NAVIGATION);
}

const CONSTITUTION = `# Principles of the payments service

Non-negotiable principles that prevail over any other practice.

1. **The spec rules.** No implementation without an approved spec; when a decision is not covered, work stops and the question is asked.
2. **Tests as a gate.** Tests are written before the implementation and no task is done until they pass.
3. **Markdown is the source of truth.** Documents are markdown and every index is derived from them.

Version 1.0
`;

const SPEC = `# Refund flow

Status: approved

## Summary

Let merchants refund a captured payment from the dashboard.

## Functional requirements

- **FR-1** (US-1): WHEN a merchant requests a refund, THE SYSTEM SHALL create it for up to the captured amount.
- **FR-2** (US-1): THE SYSTEM SHALL notify the customer by email when a refund completes.
- **FR-3** (US-2): THE SYSTEM SHALL record every refund in the audit trail.

## Open questions

- [NEEDS CLARIFICATION: Which currencies can be refunded?]

## Success criteria

Refunds complete in under two seconds.
`;

const PLAN = `# Plan

## Summary

Add a refunds module to the payments service.

## Modules

- refunds repository
- refunds API route
`;

const TASKS = `# Tasks

## Phase 1: Setup

- [x] T01 [P] (FR:—) Create the refunds module — Done when: the crate builds · commit: abc1234
- [ ] T02 (FR:—) Add the migration — Done when: the migration applies

## Phase 2: Implementation

- [x] T03 (FR-1) Create refunds from the API — Done when: the API test passes · commit: def5678
- [ ] T04 [P] (FR-1, FR-2) Send the completion email — Done when: the mail is captured
`;

function writeTree(root: string) {
  mkdirSync(join(root, "docs"), { recursive: true });
  mkdirSync(join(root, "specs", "001-refund-flow"), { recursive: true });
  writeFileSync(join(root, "docs", "constitution.md"), CONSTITUTION);
  writeFileSync(join(root, "specs", "001-refund-flow", "spec.md"), SPEC);
  writeFileSync(join(root, "specs", "001-refund-flow", "plan.md"), PLAN);
  writeFileSync(join(root, "specs", "001-refund-flow", "tasks.md"), TASKS);
}

test("the specs list, a spec with every tab and the constitution read like the design", async ({
  page,
}, testInfo) => {
  const slug = `views-${Date.now().toString(36)}`;
  await signIn(page);
  await page.goto(`${app}/onboarding`);
  await page.fill("#slug", slug);
  await page.fill("#name", `Views ${slug}`);
  await page.getByRole("button", { name: "Create organization" }).click();
  await expect(page).toHaveURL(`${app}/o/${slug}`, NAVIGATION);

  await page.getByRole("button", { name: "Create project" }).click();
  const created = page.getByRole("dialog", { name: "New project" });
  await created.getByLabel("Name").fill("Empty");
  await created.getByRole("button", { name: "Create project" }).click();
  await expect(page).toHaveURL(`${app}/p/${slug}/empty`, NAVIGATION);
  await page.goto(`${app}/p/${slug}/empty/specs`);
  await expect(page.getByRole("heading", { level: 1, name: "Specs" })).toBeVisible(NAVIGATION);
  await expect(page.getByText("No specs yet")).toBeVisible();
  await page.goto(`${app}/p/${slug}/empty/constitution`);
  await expect(page.getByText("No constitution yet")).toBeVisible(NAVIGATION);
  await page.screenshot({ path: `${evidence}/spec-views-constitution-empty.png` });

  await page.goto(`${app}/o/${slug}`);
  const tree = testInfo.outputPath("repo");
  writeTree(tree);
  await page.getByRole("button", { name: "Import SDD tree" }).click();
  const importDialog = page.getByRole("dialog", { name: "New project" });
  await importDialog.getByLabel("Name").fill("Payments");
  await importDialog.getByLabel("Choose folder").setInputFiles(tree);
  await expect(importDialog.getByText(/specs: 1, documents: 4/)).toBeVisible();
  await importDialog.getByRole("button", { name: "Create and import" }).click();
  await expect(page).toHaveURL(`${app}/p/${slug}/payments`, { timeout: 60_000 });

  const nav = page.getByRole("navigation", { name: "Main navigation" });
  await nav.getByRole("link", { name: "Specs", exact: true }).click();
  await expect(page).toHaveURL(`${app}/p/${slug}/payments/specs`, NAVIGATION);
  await expect(page.getByRole("heading", { level: 1, name: "Specs" })).toBeVisible(NAVIGATION);
  await expect(page.getByText("1 spec", { exact: true })).toBeVisible();
  await expect(page.getByRole("button", { name: "All 1" })).toBeVisible();
  const card = page.getByRole("link", { name: /001.*Refund flow/ });
  await expect(card).toBeVisible(NAVIGATION);
  await expect(card.getByText("3 requirements")).toBeVisible();
  await expect(card.getByText("2 / 4 tasks")).toBeVisible();
  await expect(card.getByText("1 open question")).toBeVisible();
  await page.screenshot({ path: `${evidence}/spec-views-list.png` });

  await page.getByRole("textbox", { name: "Search by title, slug or number" }).fill("zzz");
  await expect(page.getByText("No specs match these filters.")).toBeVisible();
  await page.getByRole("textbox", { name: "Search by title, slug or number" }).fill("refund");
  await expect(card).toBeVisible();

  await card.click();
  await expect(page).toHaveURL(`${app}/p/${slug}/payments/specs/1`, NAVIGATION);
  await expect(page.getByRole("heading", { level: 1, name: "Refund flow" })).toBeVisible(
    NAVIGATION,
  );
  await expect(page.getByRole("tab", { name: "Tasks · 2/4" })).toBeVisible();
  await expect(page.getByRole("tab", { name: "Markers · 1" })).toBeVisible();
  await expect(page.getByRole("tab", { name: "Validation" })).toHaveAttribute(
    "aria-disabled",
    "true",
  );
  await expect(page.getByText(/Revision 1 of 1 · by dev/)).toBeVisible(NAVIGATION);
  await expect(page.getByRole("region", { name: "Readiness" })).toBeVisible();
  await expect(page.getByRole("navigation", { name: "On this page" })).toBeVisible();
  await page.getByText("FR-1", { exact: true }).first().hover();
  await expect(page.getByText("Referenced by T03, T04")).toBeVisible();
  await page.screenshot({ path: `${evidence}/spec-views-spec.png` });
  await page.getByRole("tab", { name: "Raw" }).click();
  await expect(page.getByTestId("raw")).toContainText("## Functional requirements");
  await page.screenshot({ path: `${evidence}/spec-views-spec-raw.png` });

  await page.getByRole("tab", { name: "Plan" }).click();
  await expect(page).toHaveURL(`${app}/p/${slug}/payments/specs/1/plan`, NAVIGATION);
  await expect(page.getByRole("heading", { name: "Modules" })).toBeVisible(NAVIGATION);
  await page.screenshot({ path: `${evidence}/spec-views-plan.png` });

  await page.getByRole("tab", { name: /^Tasks/ }).click();
  await expect(page).toHaveURL(`${app}/p/${slug}/payments/specs/1/tasks`, NAVIGATION);
  await expect(page.getByRole("region", { name: "Phase 1: Setup" })).toBeVisible(NAVIGATION);
  await expect(page.getByText("2 of 4 done")).toBeVisible();
  await page.screenshot({ path: `${evidence}/spec-views-tasks.png` });
  await page.getByRole("button", { name: "Pending" }).click();
  await expect(page.getByText("Create refunds from the API")).toBeHidden();
  await expect(page.getByText("Add the migration")).toBeVisible();
  await page.getByRole("button", { name: "Pending" }).click();
  await page.getByRole("button", { name: "Mine" }).click();
  await expect(page.getByText("Create the refunds module")).toBeVisible();
  await expect(page.getByText("Add the migration")).toBeHidden();
  await page.getByRole("button", { name: "Parallel" }).click();
  await expect(page.getByText("Create refunds from the API")).toBeHidden();
  await page.getByRole("button", { name: "Parallel" }).click();
  await page.getByRole("button", { name: "Mine" }).click();

  await page.getByRole("tab", { name: "Traceability" }).click();
  await expect(page).toHaveURL(`${app}/p/${slug}/payments/specs/1/traceability`, NAVIGATION);
  const table = page.getByRole("table", { name: "Requirement coverage" });
  await expect(table).toBeVisible(NAVIGATION);
  await expect(table.getByRole("row")).toHaveCount(4);
  await expect(table.getByText("In progress")).toBeVisible();
  await expect(table.getByText("No tasks")).toBeVisible();
  await expect(page.getByText(/T01, T02 have no requirement/)).toBeVisible();
  await page.screenshot({ path: `${evidence}/spec-views-traceability.png` });

  await page.getByRole("tab", { name: "Research" }).click();
  await expect(page.getByText("Research is not written yet.")).toBeVisible(NAVIGATION);

  await page.getByRole("tab", { name: /^Markers/ }).click();
  await expect(page).toHaveURL(`${app}/p/${slug}/payments/specs/1/markers`, NAVIGATION);
  await expect(page.getByText("Which currencies can be refunded?")).toBeVisible(NAVIGATION);
  await page.screenshot({ path: `${evidence}/spec-views-markers.png` });

  await page.getByRole("tab", { name: "History" }).click();
  await expect(page).toHaveURL(`${app}/p/${slug}/payments/specs/1/history`, NAVIGATION);
  await expect(page.getByText(/r1 · first revision · spec.md/)).toBeVisible(NAVIGATION);
  await page.screenshot({ path: `${evidence}/spec-views-history.png` });

  await page.goto(`${app}/p/${slug}/payments/specs/1/spec`);
  await expect(page.getByRole("heading", { name: "Functional requirements" })).toBeVisible(
    NAVIGATION,
  );

  await nav.getByRole("link", { name: "Constitution" }).click();
  await expect(page).toHaveURL(`${app}/p/${slug}/payments/constitution`, NAVIGATION);
  await expect(
    page.getByRole("heading", { level: 1, name: "Principles of the payments service" }),
  ).toBeVisible(NAVIGATION);
  await expect(page.getByRole("list", { name: "Principles" }).getByRole("listitem")).toHaveCount(3);
  await expect(page.getByText("You are an owner. Amendments create a new version.")).toBeVisible();
  await expect(page.getByRole("region", { name: "Version history" })).toBeVisible();
  await page.screenshot({ path: `${evidence}/spec-views-constitution.png` });
});
