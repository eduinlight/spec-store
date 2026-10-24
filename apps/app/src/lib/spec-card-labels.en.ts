import type { SpecCardLabels } from "@workspace/ui/patterns";

export const specCardLabelsEN: SpecCardLabels = {
  liveAgents: (count: number) => `${count} ${count === 1 ? "agent" : "agents"} live`,
  openQuestions: (count: number) => `${count} ${count === 1 ? "open question" : "open questions"}`,
  requirements: (count: number) => `${count} ${count === 1 ? "requirement" : "requirements"}`,
  tasks: (done: number, total: number) => `${done} / ${total} tasks`,
  noTasks: "No tasks yet",
  written: "written",
  notWritten: "not written yet",
  updated: (when: string) => `Updated ${when}`,
};
