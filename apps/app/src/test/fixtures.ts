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
