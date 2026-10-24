import type { components } from "~/api/schema";

export type SpecTask = components["schemas"]["TaskOutput"];

export type TaskFilter = "pending" | "done" | "mine" | "parallel";
export const TASK_FILTERS: TaskFilter[] = ["pending", "done", "mine", "parallel"];

export interface TaskPhase {
  name: string;
  tasks: SpecTask[];
}

export const toggleFilter = (active: TaskFilter[], filter: TaskFilter): TaskFilter[] => {
  if (active.includes(filter)) return active.filter((item) => item !== filter);
  const exclusive: TaskFilter[] =
    filter === "pending" ? ["done"] : filter === "done" ? ["pending"] : [];
  return [...active.filter((item) => !exclusive.includes(item)), filter];
};

export const applyFilters = (tasks: SpecTask[], active: TaskFilter[], handle: string): SpecTask[] =>
  tasks.filter(
    (task) =>
      (!active.includes("pending") || !task.done) &&
      (!active.includes("done") || task.done) &&
      (!active.includes("mine") || task.completed_by?.author === handle) &&
      (!active.includes("parallel") || task.parallel),
  );

export const groupByPhase = (tasks: SpecTask[]): TaskPhase[] => {
  const phases: TaskPhase[] = [];
  for (const task of tasks) {
    const found = phases.find((phase) => phase.name === task.phase);
    if (found) found.tasks.push(task);
    else phases.push({ name: task.phase, tasks: [task] });
  }
  return phases;
};
