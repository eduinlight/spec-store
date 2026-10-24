export function mcpBaseUrl(): string {
  const configured = import.meta.env.VITE_MCP_URL;
  return (configured ? String(configured) : window.location.origin).replace(/\/+$/, "");
}

export const connectCommand = (baseUrl: string): string =>
  `claude mcp add --transport http sdd-vault ${baseUrl}/mcp`;

export const NON_AGENT_CLIENTS = ["Web app", "sddvault CLI"];

export const isAgentClient = (client: string | null | undefined): boolean =>
  Boolean(client) && !NON_AGENT_CLIENTS.includes(client as string);
