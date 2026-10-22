import { defineConfig } from "@playwright/test";

export default defineConfig({
  testDir: "e2e",
  timeout: 60_000,
  workers: 1,
  reporter: "list",
  use: {
    viewport: { width: 1280, height: 860 },
  },
});
