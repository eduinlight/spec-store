import type { IconName } from "@workspace/ui/taptap";
import type { ParseKeys } from "i18next";

export type Area = "project" | "organization" | "account" | "other";

export interface Place {
  area: Area;
  org?: string;
  project?: string;
  section: string;
}

export type NavIcon = IconName | "LockDocument";

export interface NavEntry {
  id: string;
  section: string;
  icon: NavIcon;
  label: ParseKeys;
  chord?: string;
}

export const PROFILE_PATH = "/profile";
export const TOKENS_PATH = "/account/tokens";
export const PREFERENCES_PATH = "/account/preferences";

const ORGANIZATION_ACCOUNT_SECTIONS = ["security", "billing", "audit"];
const SWITCHABLE_PROJECT_SECTIONS = [
  "active",
  "specs",
  "constitution",
  "activity",
  "members",
  "audit",
  "settings",
  "history",
];
const SWITCHABLE_ORGANIZATION_SECTIONS = ["members", "invitations", "audit", "security", "billing"];

export const PROJECT_MAIN: NavEntry[] = [
  { id: "overview", section: "", icon: "BarChart", label: "shell.nav.overview", chord: "o" },
];

export const PROJECT_SDD: NavEntry[] = [
  { id: "active", section: "active", icon: "Focus", label: "shell.nav.active", chord: "e" },
  { id: "specs", section: "specs", icon: "File", label: "shell.nav.specs", chord: "l" },
  {
    id: "constitution",
    section: "constitution",
    icon: "LockDocument",
    label: "shell.nav.constitution",
    chord: "c",
  },
];

export const PROJECT_SECTION: NavEntry[] = [
  { id: "activity", section: "activity", icon: "Time", label: "shell.nav.activity", chord: "a" },
  { id: "members", section: "members", icon: "User", label: "shell.nav.members", chord: "m" },
  { id: "audit", section: "audit", icon: "Show", label: "shell.nav.audit", chord: "u" },
  { id: "settings", section: "settings", icon: "Setting", label: "shell.nav.settings" },
];

export const ACCOUNT_ENTRIES: (NavEntry & { path: string })[] = [
  {
    id: "profile",
    section: "profile",
    icon: "User",
    label: "shell.nav.profile",
    chord: "p",
    path: PROFILE_PATH,
  },
  {
    id: "tokens",
    section: "tokens",
    icon: "Attachment",
    label: "shell.nav.tokens",
    path: TOKENS_PATH,
  },
  {
    id: "preferences",
    section: "preferences",
    icon: "Setting",
    label: "shell.nav.preferences",
    path: PREFERENCES_PATH,
  },
];

export const ORGANIZATION_ACCOUNT_ENTRIES: NavEntry[] = [
  { id: "security", section: "security", icon: "Hide", label: "shell.nav.security" },
  { id: "billing", section: "billing", icon: "PieChart", label: "shell.nav.billing" },
  { id: "org-audit", section: "audit", icon: "File", label: "shell.nav.orgAudit" },
];

export const ORGANIZATION_ENTRIES: NavEntry[] = [
  { id: "org-projects", section: "", icon: "BarChart", label: "shell.nav.projects" },
  { id: "org-members", section: "members", icon: "User", label: "shell.nav.members" },
  { id: "org-invitations", section: "invitations", icon: "Mail", label: "shell.nav.invitations" },
];

export const parsePlace = (pathname: string): Place => {
  const [head, first, second, third] = pathname.split("/").filter(Boolean);
  if (head === "p" && first && second) {
    return { area: "project", org: first, project: second, section: third ?? "" };
  }
  if (head === "o" && first) {
    return { area: "organization", org: first, section: second ?? "" };
  }
  if (head === "profile") {
    return { area: "account", section: "profile" };
  }
  if (head === "account" && first) {
    return { area: "account", section: first };
  }
  return { area: "other", section: head ?? "" };
};

export const accountMode = (place: Place): boolean =>
  place.area === "account" ||
  (place.area === "organization" && ORGANIZATION_ACCOUNT_SECTIONS.includes(place.section));

export const projectPath = (org: string, project: string, section = ""): string =>
  `/p/${org}/${project}${section ? `/${section}` : ""}`;

export const organizationPath = (org: string, section = ""): string =>
  `/o/${org}${section ? `/${section}` : ""}`;

export const isSelected = (place: Place, section: string): boolean => place.section === section;

export const switchProjectPath = (pathname: string, org: string, project: string): string => {
  const place = parsePlace(pathname);
  if (place.area === "project" && SWITCHABLE_PROJECT_SECTIONS.includes(place.section)) {
    return projectPath(org, project, place.section);
  }
  return projectPath(org, project);
};

export const switchOrganizationPath = (pathname: string, org: string): string => {
  const place = parsePlace(pathname);
  if (place.area === "account") {
    return pathname;
  }
  if (place.area === "organization" && SWITCHABLE_ORGANIZATION_SECTIONS.includes(place.section)) {
    return organizationPath(org, place.section);
  }
  if (place.area === "project" && place.section === "audit") {
    return organizationPath(org, "audit");
  }
  return organizationPath(org);
};

export const goTarget = (
  chord: string,
  context: { org?: string; project?: string },
): string | undefined => {
  const { org, project } = context;
  if (chord === "p") {
    return PROFILE_PATH;
  }
  if (org && project) {
    const entry = [...PROJECT_MAIN, ...PROJECT_SDD, ...PROJECT_SECTION].find(
      (item) => item.chord === chord,
    );
    return entry ? projectPath(org, project, entry.section) : undefined;
  }
  if (org) {
    const sections: Record<string, string> = { o: "", u: "audit", m: "members" };
    return chord in sections ? organizationPath(org, sections[chord]) : undefined;
  }
  return undefined;
};
