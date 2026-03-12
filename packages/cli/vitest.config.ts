import { mergeConfig } from "vitest/config";
import base from "../../config/vitest.config.base.ts";
import { resolve } from "path";

const __dirname = new URL(".", import.meta.url).pathname;

export default mergeConfig(base, {
  resolve: {
    alias: {
      "bun:sqlite": resolve(__dirname, "src/__stubs__/bun-sqlite.ts"),
    },
  },
  test: {
    setupFiles: [resolve(__dirname, "src/__stubs__/bun-globals.ts")],
  },
});
