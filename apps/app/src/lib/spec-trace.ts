import type { components } from "~/api/schema";

export type TraceRequirement = components["schemas"]["TraceRequirementOutput"];
export type TraceTask = components["schemas"]["TraceTaskOutput"];

export const TRACE_STATUSES = ["covered", "in_progress", "not_started", "no_tasks"] as const;
export type TraceStatus = (typeof TRACE_STATUSES)[number];

export const traceStatusOf = (value: string): TraceStatus =>
  TRACE_STATUSES.find((status) => status === value) ?? "no_tasks";

export const doneCount = (requirement: TraceRequirement): number =>
  requirement.tasks.filter((task) => task.done).length;

export const coveragePercent = (requirement: TraceRequirement): number =>
  requirement.tasks.length === 0
    ? 0
    : Math.round((doneCount(requirement) / requirement.tasks.length) * 100);

export const commitsOf = (requirement: TraceRequirement): string[] => [
  ...new Set(requirement.tasks.flatMap((task) => (task.done && task.commit ? [task.commit] : []))),
];

export const requirementsWithTasks = (requirements: TraceRequirement[]): number =>
  requirements.filter((requirement) => requirement.tasks.length > 0).length;

export const taskIdsByRequirement = (requirements: TraceRequirement[]): Map<string, string[]> =>
  new Map(
    requirements.map((requirement) => [requirement.id, requirement.tasks.map((task) => task.id)]),
  );
