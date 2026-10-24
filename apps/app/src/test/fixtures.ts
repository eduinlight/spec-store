export const me = {
  handle: "alice",
  email: "alice@example.com",
  name: "Alice",
  is_operator: false,
};

export const acme = { slug: "acme", name: "Acme Labs", role: "owner" };

export const demoProject = { address: "acme/demo", slug: "demo", name: "Demo", role: "owner" };

export const summary = (overrides: Record<string, unknown> = {}) => ({
  address: "acme/demo",
  slug: "demo",
  name: "Demo",
  role: "owner",
  archived: false,
  constitution_version: null,
  constitution_updated_at: null,
  active_spec: null,
  last_activity: null,
  open_markers: 0,
  ...overrides,
});

export const projectDetail = (overrides: Record<string, unknown> = {}) => ({
  address: "acme/demo",
  organization: "acme",
  slug: "demo",
  name: "Demo",
  repository_url: "https://github.com/acme/demo",
  role: "owner",
  archived: false,
  created_at: "2026-10-01T10:00:00.000Z",
  ...overrides,
});

export const specCard = (number: number, overrides: Record<string, unknown> = {}) => ({
  number,
  slug: `spec-${number}`,
  title: `Spec title ${number}`,
  summary: `Summary ${number}`,
  status: "approved",
  branch: `${String(number).padStart(3, "0")}-spec-${number}`,
  created_at: `2026-09-${String(number).padStart(2, "0")}T10:00:00.000Z`,
  updated_at: `2026-10-${String(number).padStart(2, "0")}T10:00:00.000Z`,
  contributors: ["alice"],
  documents: ["spec", "plan", "tasks"],
  requirement_count: 4,
  tasks_done: 2,
  tasks_total: 5,
  open_markers: 0,
  ...overrides,
});

export const overviewData = (overrides: Record<string, unknown> = {}) => ({
  days: 7,
  active_specs: 2,
  tasks_completed: 9,
  tasks_completed_previous: 6,
  coverage: { spec_number: 4, covered: 3, total: 4 },
  open_markers: 3,
  agents_active: 1,
  completed_per_day: [
    { day: "2026-10-23", by_client: [{ client: "Claude Code", count: 3 }] },
    {
      day: "2026-10-24",
      by_client: [
        { client: "Claude Code", count: 4 },
        { client: "Cursor", count: 2 },
      ],
    },
  ],
  attention: [],
  ...overrides,
});

export const activityRow = (id: number, overrides: Record<string, unknown> = {}) => ({
  audit_id: id,
  actor: "dana",
  client: "Cursor",
  action: "document.put",
  target: "004/spec@14",
  result: "ok",
  detail: null,
  at: "2026-10-24T09:00:00.000Z",
  summary_key: "document.put",
  resolved_by_audit_id: null,
  ...overrides,
});

export const nextTask = (overrides: Record<string, unknown> = {}) => ({
  id: "T13",
  parallel: true,
  frs: ["FR-1", "FR-2"],
  description: "Migrate the refunds repository",
  done_when: "the refund tests pass",
  ...overrides,
});

export const activeSpec = (overrides: Record<string, unknown> = {}) => ({
  number: 4,
  slug: "refunds",
  title: "Refund flow",
  status: "approved",
  branch: "004-refunds",
  tasks_done: 12,
  tasks_total: 20,
  current_phase: "Phase 3: Implementation",
  open_markers: 1,
  next_task: nextTask(),
  agent_prompt: "Work on project acme/demo, task T13.",
  ...overrides,
});

export const viewRoutes = () => ({
  "GET /api/v1/projects/{org}/{project}/overview": () => overviewData(),
  "GET /api/v1/projects/{org}/{project}/active": () => ({ active: null, drafts: [] }),
  "GET /api/v1/projects/{org}/{project}/activity": () => ({ rows: [], has_more: false }),
});
