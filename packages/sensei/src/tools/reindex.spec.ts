import { describe, it, expect, beforeEach, afterEach } from "vitest";
import { mkdirSync, writeFileSync, rmSync, existsSync, readFileSync } from "fs";
import { join } from "path";
import { reindexRepo } from "./reindex.js";

const TMP = "/tmp/sensei-test-reindex";

beforeEach(() => {
  mkdirSync(join(TMP, "src"), { recursive: true });
  writeFileSync(join(TMP, "package.json"), JSON.stringify({
    name: "test-app", version: "1.0.0",
    scripts: { dev: "bun src/index.ts", test: "bunx vitest" },
    dependencies: { express: "^4.0.0" }
  }));
  writeFileSync(join(TMP, "src/index.ts"),
    `export const app = "app";\nexport function startServer(port: number): void { }\n`);
  writeFileSync(join(TMP, "README.md"), "# Test App\nA test application.");
});

afterEach(() => rmSync(TMP, { recursive: true, force: true }));

describe("reindexRepo", () => {
  it("creates .index directory", async () => {
    await reindexRepo(TMP);
    expect(existsSync(join(TMP, ".index"))).toBe(true);
  });

  it("writes symbol-map.json with L0 entries", async () => {
    await reindexRepo(TMP);
    const map = JSON.parse(readFileSync(join(TMP, ".index/symbol-map.json"), "utf-8"));
    expect(Object.keys(map).length).toBeGreaterThan(0);
  });

  it("writes stack.md with detected stack", async () => {
    await reindexRepo(TMP);
    const stack = readFileSync(join(TMP, ".index/stack.md"), "utf-8");
    expect(stack).toContain("express");
  });

  it("writes shortcuts.md from package.json scripts", async () => {
    await reindexRepo(TMP);
    const shortcuts = readFileSync(join(TMP, ".index/shortcuts.md"), "utf-8");
    expect(shortcuts).toContain("bun src/index.ts");
  });

  it("creates .llmspec.yaml template if missing", async () => {
    await reindexRepo(TMP);
    expect(existsSync(join(TMP, ".llmspec.yaml"))).toBe(true);
  });

  it("does not overwrite existing .llmspec.yaml", async () => {
    writeFileSync(join(TMP, ".llmspec.yaml"), "project: my-custom-project\n");
    await reindexRepo(TMP);
    const content = readFileSync(join(TMP, ".llmspec.yaml"), "utf-8");
    expect(content).toContain("my-custom-project");
  });

  it("writes doc-index.json with file fingerprints", async () => {
    await reindexRepo(TMP);
    const index = JSON.parse(readFileSync(join(TMP, ".index/doc-index.json"), "utf-8"));
    expect(index["README.md"]).toHaveProperty("mtime");
    expect(index["README.md"]).toHaveProperty("size");
  });
});
