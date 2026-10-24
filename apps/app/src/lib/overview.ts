import type { components } from "~/api/schema";

export type OverviewData = components["schemas"]["OverviewOutput"];
export type DayCompleted = components["schemas"]["DayCompletedOutput"];

export const PERIODS = [7, 14, 30] as const;
export type Period = (typeof PERIODS)[number];
export const DEFAULT_PERIOD: Period = 7;
export const MIN_CHART_MAX = 5;
export const CHART_STEP = 5;

export const periodOf = (value: string | null): Period =>
  PERIODS.find((period) => String(period) === value) ?? DEFAULT_PERIOD;

export interface ChartData {
  days: string[];
  series: { name: string; data: number[] }[];
  max: number;
  total: number;
}

export function chartData(perDay: DayCompleted[]): ChartData {
  const totals = new Map<string, number>();
  for (const day of perDay) {
    for (const item of day.by_client) {
      totals.set(item.client, (totals.get(item.client) ?? 0) + item.count);
    }
  }
  const clients = [...totals.entries()]
    .sort((a, b) => b[1] - a[1] || a[0].localeCompare(b[0]))
    .map(([client]) => client);
  const series = clients.map((client) => ({
    name: client,
    data: perDay.map((day) => day.by_client.find((item) => item.client === client)?.count ?? 0),
  }));
  const peak = Math.max(
    0,
    ...perDay.map((day) => day.by_client.reduce((sum, item) => sum + item.count, 0)),
  );
  return {
    days: perDay.map((day) => day.day),
    series,
    max: Math.max(MIN_CHART_MAX, Math.ceil(peak / CHART_STEP) * CHART_STEP),
    total: [...totals.values()].reduce((sum, value) => sum + value, 0),
  };
}

export const coveragePercent = (coverage: OverviewData["coverage"]): number | null =>
  coverage && coverage.total > 0 ? Math.round((coverage.covered / coverage.total) * 100) : null;

export type AttentionKind = "approval_waiting" | "open_questions" | "requirement_without_work";

export const ATTENTION_TARGETS: Record<AttentionKind, string> = {
  approval_waiting: "",
  open_questions: "/markers",
  requirement_without_work: "/traceability",
};

export const isAttentionKind = (value: string): value is AttentionKind =>
  value in ATTENTION_TARGETS;

export const attentionPath = (
  base: string,
  item: OverviewData["attention"][number],
): string | null =>
  isAttentionKind(item.kind)
    ? `${base}/specs/${item.spec_number}${ATTENTION_TARGETS[item.kind]}`
    : null;
