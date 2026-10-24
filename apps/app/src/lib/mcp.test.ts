import { connectCommand, isAgentClient, mcpBaseUrl } from "./mcp";

afterEach(() => {
  vi.unstubAllEnvs();
});

test("the connect command points the agent at the MCP endpoint", () => {
  expect(connectCommand("https://mcp.example.test")).toBe(
    "claude mcp add --transport http sdd-vault https://mcp.example.test/mcp",
  );
});

test("the MCP address comes from the environment or falls back to this origin", () => {
  vi.stubEnv("VITE_MCP_URL", "https://mcp.example.test/");
  expect(mcpBaseUrl()).toBe("https://mcp.example.test");
  vi.stubEnv("VITE_MCP_URL", "");
  expect(mcpBaseUrl()).toBe(window.location.origin);
});

test("only clients other than the web app and the CLI count as agents", () => {
  expect(isAgentClient("Claude Code")).toBe(true);
  expect(isAgentClient("Web app")).toBe(false);
  expect(isAgentClient("sddvault CLI")).toBe(false);
  expect(isAgentClient(null)).toBe(false);
  expect(isAgentClient(undefined)).toBe(false);
});
