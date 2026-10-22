import { ApiFailure } from "~/api/client";

export async function attempt<T>(work: () => Promise<T>): Promise<T | { error: string }> {
  try {
    return await work();
  } catch (error) {
    if (error instanceof ApiFailure && error.status !== 401) return { error: error.message };
    throw error;
  }
}

export function errorOf(data: unknown): string | undefined {
  return data && typeof data === "object" && "error" in data
    ? String((data as { error: unknown }).error)
    : undefined;
}
