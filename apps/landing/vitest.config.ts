import { defineConfig } from "vitest/config";

export default defineConfig({
  test: {
    globals: true,
    projects: [
      {
        test: {
          name: "node",
          globals: true,
          environment: "node",
          include: ["app/**/*.test.ts"],
        },
      },
      {
        test: {
          name: "dom",
          globals: true,
          environment: "jsdom",
          include: ["app/**/*.test.tsx"],
          setupFiles: ["./app/test-setup.ts"],
        },
      },
    ],
  },
});
