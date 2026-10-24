export const SPEC_TABS = [
  "spec",
  "plan",
  "tasks",
  "traceability",
  "research",
  "validation",
  "markers",
  "history",
] as const;
export type SpecTab = (typeof SPEC_TABS)[number];

export const DOCUMENT_TABS = ["spec", "plan", "research", "validation"] as const;
export type DocumentTab = (typeof DOCUMENT_TABS)[number];

export const isDocumentTab = (value: string | undefined): value is DocumentTab =>
  DOCUMENT_TABS.some((tab) => tab === value);

export const tabPath = (specBase: string, tab: SpecTab): string =>
  tab === "spec" ? specBase : `${specBase}/${tab}`;

export const tabOf = (pathname: string, specBase: string): SpecTab => {
  const rest = pathname.startsWith(specBase) ? pathname.slice(specBase.length) : "";
  const segment = rest.split("/").filter(Boolean)[0];
  return SPEC_TABS.find((tab) => tab === segment) ?? "spec";
};
