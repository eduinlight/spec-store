import type { components } from "~/api/schema";

export type SpecCardData = components["schemas"]["SpecCardOutput"];

export const STATUS_FILTERS = [
  "all",
  "active",
  "draft",
  "clarified",
  "approved",
  "archived",
] as const;
export type StatusFilter = (typeof STATUS_FILTERS)[number];

export const SORTS = ["newest", "oldest", "updated"] as const;
export type SpecSort = (typeof SORTS)[number];

export const PAGE_SIZE = 20;

const matchesStatus = (card: SpecCardData, filter: StatusFilter): boolean => {
  if (filter === "all") return true;
  if (filter === "active") return card.status !== "archived";
  return card.status === filter;
};

export const countByStatus = (cards: SpecCardData[]): Record<StatusFilter, number> =>
  Object.fromEntries(
    STATUS_FILTERS.map((filter) => [
      filter,
      cards.filter((card) => matchesStatus(card, filter)).length,
    ]),
  ) as Record<StatusFilter, number>;

const matchesQuery = (card: SpecCardData, query: string): boolean => {
  const needle = query.trim().toLowerCase();
  if (!needle) return true;
  const padded = String(card.number).padStart(3, "0");
  return (
    card.title.toLowerCase().includes(needle) ||
    card.slug.toLowerCase().includes(needle) ||
    padded.includes(needle) ||
    String(card.number) === needle
  );
};

export const filterSpecs = (
  cards: SpecCardData[],
  status: StatusFilter,
  query: string,
): SpecCardData[] =>
  cards.filter((card) => matchesStatus(card, status) && matchesQuery(card, query));

export const sortSpecs = (cards: SpecCardData[], sort: SpecSort): SpecCardData[] => {
  const sorted = [...cards];
  if (sort === "oldest") return sorted.sort((a, b) => a.number - b.number);
  if (sort === "updated") return sorted.sort((a, b) => b.updated_at.localeCompare(a.updated_at));
  return sorted.sort((a, b) => b.number - a.number);
};

export const activeNumber = (cards: SpecCardData[]): number | null => {
  const open = cards.filter((card) => card.status !== "archived").map((card) => card.number);
  return open.length > 0 ? Math.max(...open) : null;
};
