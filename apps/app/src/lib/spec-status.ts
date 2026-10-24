import type { StatusTagStatus } from "@workspace/ui/taptap";

export const SPEC_STATUSES = ["draft", "clarified", "approved", "archived"] as const;
export type SpecStatus = (typeof SPEC_STATUSES)[number];

const SPEC_STATUS_TAGS: Record<string, StatusTagStatus> = {
  draft: "underReview",
  clarified: "currentVersion",
  approved: "approved",
  archived: "cancel",
};

export const specTag = (status: string): StatusTagStatus =>
  SPEC_STATUS_TAGS[status] ?? "underReview";

export const lifecycleIndex = (status: string): number => {
  const index = SPEC_STATUSES.indexOf(status as SpecStatus);
  return Math.max(index, 0);
};
