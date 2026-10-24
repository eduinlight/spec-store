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
