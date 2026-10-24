import type { components } from "~/api/schema";

export type ActivityRow = components["schemas"]["ActivityRowOutput"];

export const ACTIVITY_FILTERS = ["all", "tasks", "documents", "security"] as const;
export type ActivityFilter = (typeof ACTIVITY_FILTERS)[number];
export const DEFAULT_ACTIVITY_FILTER: ActivityFilter = "all";
export const ACTIVITY_PAGE_SIZE = 50;
export const OVERVIEW_ACTIVITY_ROWS = 5;
export const FRESH_ROW_MS = 2500;

export const filterOf = (value: string | null): ActivityFilter =>
  ACTIVITY_FILTERS.find((filter) => filter === value) ?? DEFAULT_ACTIVITY_FILTER;

const ACTION_KEYS = {
  "task.done": "taskDone",
  "document.put": "documentPut",
  "spec.create": "specCreate",
  "spec.status": "specStatus",
  "spec.import": "specImport",
  "constitution.put": "constitutionPut",
  "member.set": "memberSet",
  "member.remove": "memberRemove",
  "project.create": "projectCreate",
  "project.archive": "projectArchive",
  "project.restore": "projectRestore",
  "project.repo": "projectRepo",
  "project.copy": "projectCopy",
  "project.import": "projectImport",
} as const;

export type ActionKey = (typeof ACTION_KEYS)[keyof typeof ACTION_KEYS];

export const actionKeyOf = (action: string): ActionKey | null =>
  (ACTION_KEYS as Record<string, ActionKey>)[action] ?? null;

export interface Sentence {
  key: SentenceKey;
  values: Record<string, string>;
}

export type SentenceKey =
  | `summary.${ActionKey}`
  | "summary.taskDoneNoCommit"
  | "summary.projectRepoCleared"
  | "summary.fallback"
  | "denied"
  | "deniedFallback"
  | "conflict"
  | "conflictNoBase"
  | "conflictResolved"
  | "conflictResolvedNoBase";

const splitAt = (target: string): [string, string] => {
  const index = target.indexOf("@");
  return index < 0 ? [target, ""] : [target.slice(0, index), target.slice(index + 1)];
};

const revisionLabel = (revision: string) => (revision ? `r${revision}` : "");

const CONFLICT_DETAIL = /^expected (\S+), current (\S+)$/;

export function conflictOf(detail: string | null | undefined) {
  const match = CONFLICT_DETAIL.exec(detail ?? "");
  return match ? { expected: match[1] ?? "", current: match[2] ?? "" } : null;
}

function conflictSentence(row: ActivityRow, saved: ActivityRow | undefined): Sentence {
  const resolved = row.summary_key === "conflict_resolved";
  const parsed = conflictOf(row.detail);
  const noBase = !parsed || parsed.expected === "none";
  const [, savedRevision] = splitAt(saved?.target ?? "");
  const [document] = splitAt(row.target);
  const values = {
    document,
    expected: parsed?.expected ?? "",
    current: parsed?.current ?? "",
    saved: savedRevision ? ` (${revisionLabel(savedRevision)})` : "",
  };
  if (resolved) {
    return { key: noBase ? "conflictResolvedNoBase" : "conflictResolved", values };
  }
  return { key: noBase ? "conflictNoBase" : "conflict", values };
}

export function sentenceOf(row: ActivityRow, rows: ActivityRow[]): Sentence {
  if (row.summary_key === "denied") {
    return actionKeyOf(row.action)
      ? { key: "denied", values: { action: row.action, target: row.target } }
      : { key: "deniedFallback", values: { action: row.action, target: row.target } };
  }
  if (row.summary_key === "conflict" || row.summary_key === "conflict_resolved") {
    const saved = rows.find((item) => item.audit_id === row.resolved_by_audit_id);
    return conflictSentence(row, saved);
  }
  const key = actionKeyOf(row.action);
  if (!key) return { key: "summary.fallback", values: { action: row.action, target: row.target } };
  const [head, tail] = splitAt(row.target);
  switch (key) {
    case "taskDone": {
      const [spec = "", task = head] = head.split("/");
      return {
        key: tail ? "summary.taskDone" : "summary.taskDoneNoCommit",
        values: { task, commit: tail, spec },
      };
    }
    case "documentPut":
      return {
        key: "summary.documentPut",
        values: { document: head, revision: revisionLabel(tail) },
      };
    case "constitutionPut":
      return {
        key: "summary.constitutionPut",
        values: { version: head, revision: revisionLabel(tail) },
      };
    case "specStatus": {
      const [number = "", change = ""] = head.split(":");
      const [from = "", to = ""] = change.split("→");
      return { key: "summary.specStatus", values: { number, from, to } };
    }
    case "memberSet": {
      const [handle = "", role = ""] = head.split(":");
      return { key: "summary.memberSet", values: { handle, role } };
    }
    case "projectRepo":
      return row.target.endsWith(" cleared")
        ? { key: "summary.projectRepoCleared", values: {} }
        : { key: "summary.projectRepo", values: { target: row.target } };
    default:
      return { key: `summary.${key}`, values: { target: row.target, branch: row.target } };
  }
}

export const mergeRows = (...pages: ActivityRow[][]): ActivityRow[] => {
  const byId = new Map<number, ActivityRow>();
  for (const page of pages) {
    for (const row of page) byId.set(row.audit_id, row);
  }
  return [...byId.values()].sort((a, b) => b.audit_id - a.audit_id);
};

export const oldestId = (rows: ActivityRow[]): number | undefined =>
  rows.length === 0 ? undefined : Math.min(...rows.map((row) => row.audit_id));
