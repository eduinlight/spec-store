export const connectCommand = (mcpUrl: string): string =>
  `claude mcp add --transport http sdd-vault ${mcpUrl}`;

export const mcpUrlOf = (me: { mcp_url?: string | null } | null | undefined): string =>
  me?.mcp_url ?? `${window.location.origin}/mcp`;

export const NON_AGENT_CLIENTS = ["Web app", "sddvault CLI"];

export const isAgentClient = (client: string | null | undefined): boolean =>
  Boolean(client) && !NON_AGENT_CLIENTS.includes(client as string);
