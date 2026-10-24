import type {
  ShellActiveSpec,
  ShellOrganization,
  ShellPerson,
  ShellProject,
  ShellSpec,
} from "./types";

export interface RawShellData {
  me: {
    handle: string;
    name: string;
    email: string;
    is_operator: boolean;
    mcp_url?: string | null;
  };
  organizations: { slug: string; name: string; role?: string | null }[];
  projects: { slug: string; name: string; role?: string | null }[];
  status: {
    specs: { number: number; slug: string; branch: string; status: string }[];
    active?: {
      number: number;
      slug: string;
      branch: string;
      next_task?: { id: string; description: string } | null;
    } | null;
  } | null;
}

export interface ShellData {
  person: ShellPerson;
  organizations: ShellOrganization[];
  projects: ShellProject[];
  specs: ShellSpec[];
  active?: ShellActiveSpec;
}

export const toShellData = (raw: RawShellData): ShellData => ({
  person: {
    handle: raw.me.handle,
    name: raw.me.name,
    email: raw.me.email,
    isOperator: raw.me.is_operator,
  },
  organizations: raw.organizations,
  projects: raw.projects,
  specs: raw.status?.specs ?? [],
  active: raw.status?.active
    ? {
        number: raw.status.active.number,
        slug: raw.status.active.slug,
        branch: raw.status.active.branch,
        nextTask: raw.status.active.next_task ?? null,
      }
    : undefined,
});
