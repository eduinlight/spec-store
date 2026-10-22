import { redirectDocument } from "react-router";
import { ApiFailure, api, unwrap } from "~/api/client";

export function signInRedirect(request: Request): Response {
  const url = new URL(request.url);
  return redirectDocument(`/auth/login?return_to=${encodeURIComponent(url.pathname + url.search)}`);
}

export async function guard<T>(request: Request, load: () => Promise<T>): Promise<T> {
  try {
    return await load();
  } catch (error) {
    if (error instanceof ApiFailure && error.status === 401) throw signInRedirect(request);
    throw error;
  }
}

export async function requireSession(request: Request) {
  return guard(request, async () => unwrap(await api.GET("/api/v1/me")));
}
