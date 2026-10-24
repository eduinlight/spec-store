import tailwindcss from "@tailwindcss/vite";
import react from "@vitejs/plugin-react";
import { defineConfig } from "vitest/config";

const api = process.env.API_URL ?? "http://localhost:54000";
const port = Number(new URL(process.env.APP_URL ?? "http://localhost:5173").port || 5173);

export default defineConfig({
  plugins: [tailwindcss(), react()],
  resolve: {
    tsconfigPaths: true,
  },
  server: {
    port,
    proxy: {
      "/api": api,
      "/auth": api,
    },
  },
  test: {
    environment: "jsdom",
    globals: true,
    setupFiles: ["./src/test/setup.ts"],
    env: { TZ: "America/New_York" },
    include: ["src/**/*.test.{ts,tsx}"],
  },
});
