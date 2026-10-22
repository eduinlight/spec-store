import createClient from "openapi-fetch";
import type { paths } from "./schema";

export const REQUEST_HEADER = "X-Spec-Store-Request";

export const api = createClient<paths>({
  baseUrl: typeof window === "undefined" ? "" : window.location.origin,
  credentials: "same-origin",
  headers: { [REQUEST_HEADER]: "1" },
  fetch: (request) => globalThis.fetch(request),
});

export class ApiFailure extends Error {
  status: number;
  code: string;

  constructor(status: number, code: string, message: string) {
    super(message);
    this.status = status;
    this.code = code;
  }
}

interface Result<T> {
  data?: T;
  error?: unknown;
  response: Response;
}

export function failure(response: Response, error: unknown): ApiFailure {
  const body = (error ?? {}) as { code?: string; message?: string };
  return new ApiFailure(response.status, body.code ?? "error", body.message ?? response.statusText);
}

export function unwrap<T>(result: Result<T>): T {
  if (result.response.ok) return result.data as T;
  throw failure(result.response, result.error);
}

export function succeeded(result: Result<unknown>): void {
  if (!result.response.ok) throw failure(result.response, result.error);
}
