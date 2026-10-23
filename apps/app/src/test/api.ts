import { vi } from "vitest";

export type Handler = (request: Request, params: Record<string, string>) => unknown;

export interface MockApi {
  calls: { method: string; path: string; body: unknown }[];
}

function matcher(pattern: string): (path: string) => Record<string, string> | null {
  const names: string[] = [];
  const regex = new RegExp(
    `^${pattern.replace(/\{(\w+)\}/g, (_, name: string) => {
      names.push(name);
      return "([^/]+)";
    })}$`,
  );
  return (path) => {
    const match = regex.exec(path);
    if (!match) return null;
    return Object.fromEntries(
      names.map((name, index) => [name, decodeURIComponent(match[index + 1] ?? "")]),
    );
  };
}

export function json(body: unknown, status = 200): Response {
  return new Response(body === undefined ? null : JSON.stringify(body), {
    status,
    headers: { "content-type": "application/json" },
  });
}

export function mockApi(routes: Record<string, Handler>): MockApi {
  const compiled = Object.entries(routes).map(([key, handler]) => {
    const [method, pattern] = key.split(" ");
    return { method, match: matcher(pattern ?? ""), handler };
  });
  const api: MockApi = { calls: [] };
  vi.stubGlobal(
    "fetch",
    vi.fn(async (input: RequestInfo | URL, init?: RequestInit) => {
      const request =
        input instanceof Request
          ? input
          : new Request(new URL(String(input), "http://app.test"), init);
      const url = new URL(request.url, "http://app.test");
      const text = request.method === "GET" ? "" : await request.clone().text();
      api.calls.push({
        method: request.method,
        path: url.pathname + url.search,
        body: text ? JSON.parse(text) : null,
      });
      for (const route of compiled) {
        if (route.method !== request.method) continue;
        const params = route.match(url.pathname);
        if (!params) continue;
        const result = await route.handler(request, params);
        return result instanceof Response ? result : json(result);
      }
      return json(
        { code: "not_found", message: `${request.method} ${url.pathname} not mocked` },
        404,
      );
    }),
  );
  return api;
}
