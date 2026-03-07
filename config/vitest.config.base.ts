import { defineConfig } from "vitest/config";

export default defineConfig({
  test: {
    include: ["src/**/*.spec.ts"],
    environment: "node",
    coverage: {
      reporter: ["text", "lcov", "html"],
    },
    projects: [
      { extends: true, test: { name: "server", root: "packages/server" } },
      { extends: true, test: { name: "tools", root: "packages/tools" } },
      { extends: true, test: { name: "cli", root: "packages/cli" } },
      { extends: true, test: { name: "mcp", root: "packages/mcp" } },
      { extends: true, test: { name: "shared", root: "packages/shared" } },
    ],
  },
});
