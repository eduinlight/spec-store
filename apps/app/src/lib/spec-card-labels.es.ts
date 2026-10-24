import type { SpecCardLabels } from "@workspace/ui/patterns";

export const specCardLabelsES: SpecCardLabels = {
  liveAgents: (count: number) => `${count} ${count === 1 ? "agente activo" : "agentes activos"}`,
  openQuestions: (count: number) =>
    `${count} ${count === 1 ? "pregunta abierta" : "preguntas abiertas"}`,
  requirements: (count: number) => `${count} ${count === 1 ? "requisito" : "requisitos"}`,
  tasks: (done: number, total: number) => `${done} / ${total} tareas`,
  noTasks: "Sin tareas todavía",
  written: "escrito",
  notWritten: "sin escribir todavía",
  updated: (when: string) => `Actualizada ${when}`,
};
