import { defineConfig } from "@playwright/test";

const api = process.env.API_URL ?? "http://localhost:54000";
const app = process.env.APP_URL ?? "http://localhost:5173";

const apiEnv = {
  DATABASE_URL: process.env.DATABASE_URL ?? "postgres://platform:${PLATFORM_DB_PASSWORD}@localhost:55432/platform",
  KEYCLOAK_ISSUER: process.env.KEYCLOAK_ISSUER ?? "http://localhost:58080/realms/spec-store",
  KEYCLOAK_APP_CLIENT_SECRET: process.env.KEYCLOAK_APP_CLIENT_SECRET ?? "${KEYCLOAK_APP_CLIENT_SECRET}",
  KEYCLOAK_ADMIN_CLIENT_SECRET: process.env.KEYCLOAK_ADMIN_CLIENT_SECRET ?? "${KEYCLOAK_ADMIN_CLIENT_SECRET}",
  SESSION_SEALING_KEY:
    process.env.SESSION_SEALING_KEY ?? "${SESSION_SEALING_KEY}",
  APP_URL: app,
  LANDING_URL: process.env.E2E_LANDING_URL ?? app,
  API_BIND: process.env.API_BIND ?? "127.0.0.1:54000",
  SMTP_HOST: process.env.SMTP_HOST ?? "localhost",
  SMTP_PORT: process.env.SMTP_PORT ?? "51025",
  SMTP_FROM: process.env.SMTP_FROM ?? "no-reply@spec-store.local",
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
      command: "cargo run -q --locked -p spec-store-api",
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
  ],
});
