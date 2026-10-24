import { defineConfig } from "@playwright/test";

function required(name: string): string {
  const value = process.env[name];
  if (!value) {
    throw new Error(`${name} is not set; run the browser tests with make e2e`);
  }
  return value;
}

const api = process.env.API_URL ?? "http://localhost:54000";
const app = process.env.APP_URL ?? "http://localhost:5173";

const apiEnv = {
  DATABASE_URL: required("DATABASE_URL"),
  KEYCLOAK_ISSUER: process.env.KEYCLOAK_ISSUER ?? "http://localhost:58080/realms/sdd-vault",
  KEYCLOAK_APP_CLIENT_SECRET: required("KEYCLOAK_APP_CLIENT_SECRET"),
  KEYCLOAK_ADMIN_CLIENT_SECRET: required("KEYCLOAK_ADMIN_CLIENT_SECRET"),
  SESSION_SEALING_KEY: required("SESSION_SEALING_KEY"),
  APP_URL: app,
  LANDING_URL: process.env.E2E_LANDING_URL ?? app,
  API_BIND: process.env.API_BIND ?? "127.0.0.1:54000",
  SMTP_HOST: process.env.SMTP_HOST ?? "localhost",
  SMTP_PORT: process.env.SMTP_PORT ?? "51025",
  SMTP_FROM: process.env.SMTP_FROM ?? "no-reply@sddvault.local",
  ...(process.env.REDIS_URL ? { REDIS_URL: process.env.REDIS_URL } : {}),
};

export default defineConfig({
  testDir: "e2e",
  timeout: 90_000,
  workers: 1,
  reporter: "list",
  use: {
    viewport: { width: 1280, height: 860 },
  },
  webServer: [
    {
      command: "cargo run -q --locked -p sdd-vault-api",
      url: `${api}/healthz`,
      env: apiEnv,
      reuseExistingServer: true,
      timeout: 300_000,
    },
    {
      command: "pnpm --filter app dev --strictPort",
      url: app,
      env: { API_URL: api },
      reuseExistingServer: true,
      timeout: 120_000,
    },
    {
      command: "pnpm --filter landing dev --strictPort",
      url: process.env.LANDING_URL ?? "http://localhost:5174",
      env: { VITE_APP_URL: app },
      reuseExistingServer: true,
      timeout: 120_000,
    },
  ],
});
