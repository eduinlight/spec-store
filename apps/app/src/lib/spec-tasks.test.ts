import { applyFilters, groupByPhase, type SpecTask, toggleFilter } from "./spec-tasks";

const task = (id: string, overrides: Partial<SpecTask> = {}): SpecTask => ({
  id,
  phase: "Phase 1",
  parallel: false,
  frs: [],
  description: id,
  done_when: "",
  done: false,
  commit: null,
  line: 1,
  completed_by: null,
  ...overrides,
});

test("pending and done exclude each other while mine and parallel combine", () => {
  expect(toggleFilter(["pending", "mine"], "done")).toEqual(["mine", "done"]);
  expect(toggleFilter(["done"], "pending")).toEqual(["pending"]);
  expect(toggleFilter(["mine"], "parallel")).toEqual(["mine", "parallel"]);
  expect(toggleFilter(["mine", "parallel"], "mine")).toEqual(["parallel"]);
});

test("filters keep the tasks matching every active filter", () => {
  const mine = { author: "alice", client: null, at: "2026-10-01T00:00:00Z", revision: 1 };
  const tasks = [
    task("T01", { done: true, completed_by: mine }),
    task("T02", { parallel: true }),
    task("T03", { done: true, parallel: true }),
  ];
  const ids = (active: Parameters<typeof applyFilters>[1]) =>
    applyFilters(tasks, active, "alice").map((item) => item.id);
  expect(ids([])).toEqual(["T01", "T02", "T03"]);
  expect(ids(["pending"])).toEqual(["T02"]);
  expect(ids(["done"])).toEqual(["T01", "T03"]);
  expect(ids(["mine"])).toEqual(["T01"]);
  expect(ids(["done", "parallel"])).toEqual(["T03"]);
});

test("tasks are grouped by phase in order of appearance", () => {
  const groups = groupByPhase([task("T01"), task("T02", { phase: "Phase 2" }), task("T03")]);
  expect(groups.map((group) => [group.name, group.tasks.map((item) => item.id)])).toEqual([
    ["Phase 1", ["T01", "T03"]],
    ["Phase 2", ["T02"]],
  ]);
});
