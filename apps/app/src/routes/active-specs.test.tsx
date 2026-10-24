import { screen, waitFor, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { afterEach } from "vitest";
import { mockApi } from "~/test/api";
import { acme, activeSpec, me, nextTask, projectDetail } from "~/test/fixtures";
import { renderAt } from "~/test/render";

afterEach(() => {
  vi.unstubAllGlobals();
  vi.restoreAllMocks();
});

const draft = (overrides: Record<string, unknown> = {}) => ({
  number: 5,
  slug: "partial-refunds",
  title: "Partial refunds",
  summary: "Let merchants refund part of a payment.",
  markers: [
    {
      kind: "clarification",
      document_kind: "spec",
      heading: "Open questions",
      text: "Can a refund be split across cards?",
      line: 12,
    },
  ],
  ...overrides,
});

function setup(view: unknown) {
  return mockApi({
    "GET /api/v1/me": () => me,
    "GET /api/v1/orgs": () => ({ organizations: [acme] }),
    "GET /api/v1/orgs/{org}/projects": () => ({ projects: [] }),
    "GET /api/v1/projects/{org}/{project}": () => projectDetail(),
    "GET /api/v1/projects/{org}/{project}/active": () => view,
  });
}

test("the active spec shows its status, branch, progress, phase and open markers", async () => {
  setup({ active: activeSpec(), drafts: [] });
  renderAt("/p/acme/demo/active");
  const card = await screen.findByRole("region", { name: "Active spec" });
  expect(within(card).getByRole("link", { name: "Refund flow" })).toHaveAttribute(
    "href",
    "/p/acme/demo/specs/4",
  );
  expect(within(card).getByText("Approved")).toBeInTheDocument();
  expect(within(card).getByText("⑂ 004-refunds")).toBeInTheDocument();
  expect(within(card).getByText("12 / 20 tasks")).toBeInTheDocument();
  expect(within(card).getByText("Phase 3: Implementation")).toBeInTheDocument();
  expect(within(card).getByRole("progressbar", { name: "Progress" })).toHaveAttribute(
    "aria-valuenow",
    "12",
  );
});

test("the next task shows its id, parallel flag, requirements, description and done-when", async () => {
  setup({ active: activeSpec(), drafts: [] });
  renderAt("/p/acme/demo/active");
  const block = await screen.findByRole("region", { name: "Next task" });
  expect(within(block).getByText("T13")).toBeInTheDocument();
  expect(within(block).getByText("P")).toBeInTheDocument();
  expect(within(block).getByText("FR-1")).toBeInTheDocument();
  expect(within(block).getByText("FR-2")).toBeInTheDocument();
  expect(within(block).getByText("Migrate the refunds repository")).toBeInTheDocument();
  expect(within(block).getByText("Done when: the refund tests pass")).toBeInTheDocument();
});

test("a task that is not parallel has no P tag", async () => {
  setup({ active: activeSpec({ next_task: nextTask({ parallel: false, frs: [] }) }), drafts: [] });
  renderAt("/p/acme/demo/active");
  const block = await screen.findByRole("region", { name: "Next task" });
  expect(within(block).queryByText("P")).toBeNull();
});

test("Copy agent prompt puts the prompt on the clipboard and confirms", async () => {
  setup({ active: activeSpec(), drafts: [] });
  const user = userEvent.setup();
  renderAt("/p/acme/demo/active");
  await user.click(await screen.findByRole("button", { name: "Copy agent prompt" }));
  expect(await navigator.clipboard.readText()).toBe("Work on project acme/demo, task T13.");
  expect(await screen.findByText("Agent prompt copied.")).toBeInTheDocument();
});

test("a clipboard that refuses is reported instead of failing silently", async () => {
  setup({ active: activeSpec(), drafts: [] });
  const user = userEvent.setup();
  renderAt("/p/acme/demo/active");
  const button = await screen.findByRole("button", { name: "Copy agent prompt" });
  vi.spyOn(navigator.clipboard, "writeText").mockRejectedValue(new Error("denied"));
  await user.click(button);
  expect(
    await screen.findByText(
      "The prompt could not be copied. Copy it by hand from the spec instead.",
    ),
  ).toBeInTheDocument();
});

test("All tasks opens the Tasks tab of the spec", async () => {
  setup({ active: activeSpec(), drafts: [] });
  const { router } = renderAt("/p/acme/demo/active");
  await userEvent.click(await screen.findByRole("button", { name: "All tasks" }));
  await waitFor(() => expect(router.state.location.pathname).toBe("/p/acme/demo/specs/4/tasks"));
});

test("a spec with every task done says so instead of offering a prompt", async () => {
  setup({ active: activeSpec({ next_task: null, agent_prompt: null }), drafts: [] });
  renderAt("/p/acme/demo/active");
  expect(await screen.findByText("Every task of this spec is done.")).toBeInTheDocument();
  expect(screen.queryByRole("button", { name: "Copy agent prompt" })).toBeNull();
});

test("a draft shows its summary and open questions with a link to answer them", async () => {
  setup({ active: null, drafts: [draft()] });
  renderAt("/p/acme/demo/active");
  const card = await screen.findByRole("region", { name: "005 Partial refunds" });
  expect(within(card).getByText("Let merchants refund part of a payment.")).toBeInTheDocument();
  expect(within(card).getByText("Can a refund be split across cards?")).toBeInTheDocument();
  expect(within(card).getByText("Spec · Open questions")).toBeInTheDocument();
  expect(within(card).getByRole("link", { name: "Answer questions" })).toHaveAttribute(
    "href",
    "/p/acme/demo/specs/5/markers",
  );
  expect(within(card).getByText("⑂ 005-partial-refunds")).toBeInTheDocument();
});

test("a draft without open questions says there are none", async () => {
  setup({ active: null, drafts: [draft({ markers: [] })] });
  renderAt("/p/acme/demo/active");
  expect(await screen.findByText("No open questions.")).toBeInTheDocument();
  expect(screen.queryByRole("link", { name: "Answer questions" })).toBeNull();
});

test("without any spec the page shows an empty state", async () => {
  setup({ active: null, drafts: [] });
  const { router } = renderAt("/p/acme/demo/active");
  expect(await screen.findByText("No active spec yet")).toBeInTheDocument();
  await userEvent.click(screen.getByRole("button", { name: "Go to specs" }));
  await waitFor(() => expect(router.state.location.pathname).toBe("/p/acme/demo/specs"));
});

test("All specs opens the specs list", async () => {
  setup({ active: activeSpec(), drafts: [] });
  const { router } = renderAt("/p/acme/demo/active");
  await userEvent.click(await screen.findByRole("button", { name: "All specs" }));
  await waitFor(() => expect(router.state.location.pathname).toBe("/p/acme/demo/specs"));
});

test("the page speaks Spanish", async () => {
  setup({ active: activeSpec(), drafts: [draft()] });
  renderAt("/p/acme/demo/active", "es");
  expect(await screen.findByRole("heading", { name: "Specs activas" })).toBeInTheDocument();
  expect(screen.getByRole("button", { name: "Copiar prompt para agente" })).toBeInTheDocument();
  expect(screen.getByText("Hecha cuando: the refund tests pass")).toBeInTheDocument();
  expect(screen.getByRole("link", { name: "Responder preguntas" })).toBeInTheDocument();
});
