export interface ShellOrganization {
  slug: string;
  name: string;
  role?: string | null;
  guest?: boolean;
}

export interface ShellProject {
  slug: string;
  name: string;
  role?: string | null;
}

export interface ShellSpec {
  number: number;
  slug: string;
  branch: string;
  status: string;
}

export interface ShellNextTask {
  id: string;
  description: string;
}

export interface ShellActiveSpec {
  number: number;
  slug: string;
  branch: string;
  nextTask?: ShellNextTask | null;
}

export interface ShellPerson {
  handle: string;
  name: string;
  email: string;
  isOperator: boolean;
}
