export const APP_URL = import.meta.env.VITE_APP_URL ?? "https://app-sddvault.eduindev.com";

export const SIGN_UP_HREF = `${APP_URL}/auth/login?action=register`;
export const SIGN_IN_HREF = `${APP_URL}/auth/login`;
export const DEMO_HREF = "mailto:hello@eduindev.com?subject=SDD%20Vault%20demo";
export const SECURITY_HREF = "mailto:security@eduindev.com";

export const MCP_COMMAND =
  "claude mcp add --transport http sdd-vault https://mcp-sddvault.eduindev.com/mcp";
export const IMPORT_COMMAND = "sddvault import acme/payments . --dry-run";
export const EXPORT_COMMAND = "sddvault export acme/payments ./out";
