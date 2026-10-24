import {
  accountMode,
  isSelected,
  parsePlace,
  projectPath,
  switchOrganizationPath,
  switchProjectPath,
} from "./navigation";

test("parsePlace reads the area, organization, project and section from a path", () => {
  expect(parsePlace("/p/acme/demo")).toEqual({
    area: "project",
    org: "acme",
    project: "demo",
    section: "",
  });
  expect(parsePlace("/p/acme/demo/specs/4/plan")).toMatchObject({
    area: "project",
    section: "specs",
  });
  expect(parsePlace("/o/acme/members")).toEqual({
    area: "organization",
    org: "acme",
    section: "members",
  });
  expect(parsePlace("/profile")).toEqual({ area: "account", section: "profile" });
  expect(parsePlace("/account/tokens")).toEqual({ area: "account", section: "tokens" });
  expect(parsePlace("/onboarding")).toEqual({ area: "other", section: "onboarding" });
  expect(parsePlace("/")).toEqual({ area: "other", section: "" });
});

test("account mode covers the account screens and the organization settings screens", () => {
  expect(accountMode(parsePlace("/profile"))).toBe(true);
  expect(accountMode(parsePlace("/account/preferences"))).toBe(true);
  expect(accountMode(parsePlace("/o/acme/security"))).toBe(true);
  expect(accountMode(parsePlace("/o/acme/billing"))).toBe(true);
  expect(accountMode(parsePlace("/o/acme/audit"))).toBe(true);
  expect(accountMode(parsePlace("/o/acme"))).toBe(false);
  expect(accountMode(parsePlace("/o/acme/members"))).toBe(false);
  expect(accountMode(parsePlace("/p/acme/demo/audit"))).toBe(false);
});

test("switching project keeps the same kind of screen when it exists", () => {
  expect(switchProjectPath("/p/acme/demo/audit", "acme", "other")).toBe("/p/acme/other/audit");
  expect(switchProjectPath("/p/acme/demo/specs/4/plan", "acme", "other")).toBe(
    "/p/acme/other/specs",
  );
  expect(switchProjectPath("/p/acme/demo/revisions/9/diff", "acme", "other")).toBe("/p/acme/other");
  expect(switchProjectPath("/p/acme/demo", "acme", "other")).toBe("/p/acme/other");
  expect(switchProjectPath("/o/acme", "acme", "other")).toBe("/p/acme/other");
  expect(switchProjectPath("/profile", "acme", "other")).toBe("/p/acme/other");
});

test("switching organization keeps the same kind of screen when it exists", () => {
  expect(switchOrganizationPath("/o/acme/members", "beta")).toBe("/o/beta/members");
  expect(switchOrganizationPath("/o/acme/audit", "beta")).toBe("/o/beta/audit");
  expect(switchOrganizationPath("/o/acme/projects/new", "beta")).toBe("/o/beta");
  expect(switchOrganizationPath("/p/acme/demo/audit", "beta")).toBe("/o/beta/audit");
  expect(switchOrganizationPath("/p/acme/demo/specs", "beta")).toBe("/o/beta");
  expect(switchOrganizationPath("/profile", "beta")).toBe("/profile");
  expect(switchOrganizationPath("/onboarding", "beta")).toBe("/o/beta");
});

test("a sidebar item is selected for its screen and the screens below it", () => {
  const specs = parsePlace("/p/acme/demo/specs/4/plan");
  expect(isSelected(specs, "specs")).toBe(true);
  expect(isSelected(specs, "")).toBe(false);
  expect(isSelected(parsePlace("/p/acme/demo"), "")).toBe(true);
  expect(isSelected(parsePlace("/p/acme/demo/settings"), "")).toBe(false);
  expect(projectPath("acme", "demo")).toBe("/p/acme/demo");
  expect(projectPath("acme", "demo", "audit")).toBe("/p/acme/demo/audit");
});
