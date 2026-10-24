export const PAGE_SIZES = [10, 25, 50, 100] as const;
export const PERIODS = ["day", "week", "month", "all"] as const;
export const RESULTS = ["ok", "denied", "conflict"] as const;
export const DEFAULT_PAGE_SIZE = 25;

export type Period = (typeof PERIODS)[number];
export type AuditSort = "newest" | "oldest";

export interface AuditView {
  q: string;
  actors: string[];
  clients: string[];
  actions: string[];
  results: string[];
  period: Period;
  sort: AuditSort;
  page: number;
  pageSize: number;
}

const positive = (value: string | null, fallback: number): number => {
  const parsed = Number(value);
  return Number.isInteger(parsed) && parsed >= 1 ? parsed : fallback;
};

export function parseView(params: URLSearchParams): AuditView {
  const period = params.get("period");
  const size = positive(params.get("page_size"), DEFAULT_PAGE_SIZE);
  return {
    q: params.get("q") ?? "",
    actors: params.getAll("actor"),
    clients: params.getAll("client"),
    actions: params.getAll("action"),
    results: params.getAll("result"),
    period: PERIODS.find((item) => item === period) ?? "all",
    sort: params.get("sort") === "oldest" ? "oldest" : "newest",
    page: positive(params.get("page"), 1),
    pageSize: PAGE_SIZES.find((item) => item === size) ?? DEFAULT_PAGE_SIZE,
  };
}

export function filterParams(view: AuditView): URLSearchParams {
  const params = new URLSearchParams();
  if (view.q.trim() !== "") params.set("q", view.q.trim());
  for (const actor of view.actors) params.append("actor", actor);
  for (const client of view.clients) params.append("client", client);
  for (const action of view.actions) params.append("action", action);
  for (const result of view.results) params.append("result", result);
  if (view.period !== "all") params.set("period", view.period);
  if (view.sort !== "newest") params.set("sort", view.sort);
  return params;
}

export function viewParams(view: AuditView): URLSearchParams {
  const params = filterParams(view);
  if (view.page !== 1) params.set("page", String(view.page));
  if (view.pageSize !== DEFAULT_PAGE_SIZE) params.set("page_size", String(view.pageSize));
  return params;
}

export function apiQuery(view: AuditView) {
  return {
    q: view.q.trim() === "" ? undefined : view.q.trim(),
    actor: view.actors,
    client: view.clients,
    action: view.actions,
    result: view.results,
    period: view.period,
    sort: view.sort,
    page: view.page,
    page_size: view.pageSize,
  };
}

export function exportUrl(base: string, view: AuditView, ids: number[]): string {
  const params = ids.length > 0 ? new URLSearchParams({ ids: ids.join(",") }) : filterParams(view);
  const query = params.toString();
  return query === "" ? `${base}/audit/export.csv` : `${base}/audit/export.csv?${query}`;
}

export const isFiltered = (view: AuditView): boolean =>
  view.q.trim() !== "" ||
  view.actors.length > 0 ||
  view.clients.length > 0 ||
  view.actions.length > 0 ||
  view.results.length > 0 ||
  view.period !== "all";
