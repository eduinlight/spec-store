import { connectCommand, isAgentClient, mcpUrlOf } from "./mcp";

test("the connect command points the agent at the MCP endpoint", () => {
  expect(connectCommand("https://mcp.example.test/mcp")).toBe(
    "claude mcp add --transport http sdd-vault https://mcp.example.test/mcp",
  );
});

test("the MCP address comes from the signed-in person's API answer or falls back to this origin", () => {
  expect(mcpUrlOf({ mcp_url: "https://mcp.example.test/mcp" })).toBe(
    "https://mcp.example.test/mcp",
  );
  expect(mcpUrlOf({ mcp_url: null })).toBe(`${window.location.origin}/mcp`);
  expect(mcpUrlOf(null)).toBe(`${window.location.origin}/mcp`);
});

test("only clients other than the web app and the CLI count as agents", () => {
  expect(isAgentClient("Claude Code")).toBe(true);
  expect(isAgentClient("Web app")).toBe(false);
  expect(isAgentClient("sddvault CLI")).toBe(false);
  expect(isAgentClient(null)).toBe(false);
  expect(isAgentClient(undefined)).toBe(false);
});
