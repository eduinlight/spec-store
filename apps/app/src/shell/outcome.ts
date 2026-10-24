import { message } from "@workspace/ui/taptap";
import { useEffect } from "react";

interface TrackedFetcher {
  data: unknown;
}

export type OutcomeText = (done: string, data: Record<string, unknown>) => string | undefined;

const doneOf = (data: unknown): { done: string; data: Record<string, unknown> } | null =>
  data && typeof data === "object" && "done" in data && typeof data.done === "string"
    ? { done: data.done, data: data as Record<string, unknown> }
    : null;

export function useOutcomeToast(fetcher: TrackedFetcher, text: OutcomeText) {
  const outcome = doneOf(fetcher.data);
  // biome-ignore lint/correctness/useExhaustiveDependencies: a toast per finished action, keyed on the action result object
  useEffect(() => {
    if (!outcome) {
      return;
    }
    const content = text(outcome.done, outcome.data);
    if (content) {
      message.success(content);
    }
  }, [fetcher.data]);
}
