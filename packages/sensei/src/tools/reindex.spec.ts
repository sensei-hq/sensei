import { describe, it, expect, beforeEach, afterEach } from "vitest";
import { mkdirSync, writeFileSync, rmSync, existsSync, readFileSync } from "fs";
import { join } from "path";
import { reindexRepo } from "./reindex.js";

const TMP = "/tmp/sensei-test-reindex";

function writePkg(overrides: Record<string, unknown> = {}) {
  writeFileSync(join(TMP, "package.json"), JSON.stringify({
    name: "test-app", version: "1.0.0",
    scripts: { dev: "bun src/index.ts", test: "bunx vitest" },
    dependencies: { express: "^4.0.0" },
    ...overrides,
  }));
}

beforeEach(() => {
  mkdirSync(join(TMP, "src"), { recursive: true });
  writePkg();
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

  it("writes doc-index.json with new schema (files key)", async () => {
    await reindexRepo(TMP);
    const index = JSON.parse(readFileSync(join(TMP, ".index/doc-index.json"), "utf-8"));
    expect(index.files["README.md"]).toHaveProperty("mtime");
    expect(index.files["README.md"]).toHaveProperty("size");
  });

  it("returns IndexSummary with correct counts on first run", async () => {
    const summary = await reindexRepo(TMP);
    expect(summary.forced).toBe(true);
    expect(summary.added).toBeGreaterThan(0);
    expect(typeof summary.unchanged).toBe("number");
  });

  it("incremental: unchanged file not re-added when run twice", async () => {
    await reindexRepo(TMP);
    const summary2 = await reindexRepo(TMP);
    // No git in TMP, so mtime fallback: file unchanged → unchanged count > 0
    expect(summary2.forced).toBe(false);
    expect(summary2.added).toBe(0);
  });

  it("incremental: new file detected on second run", async () => {
    await reindexRepo(TMP);
    writeFileSync(join(TMP, "src/new.ts"), "export function newFn(): void {}\n");
    const summary2 = await reindexRepo(TMP);
    expect(summary2.added).toBe(1);
    const map = JSON.parse(readFileSync(join(TMP, ".index/symbol-map.json"), "utf-8"));
    expect(map["src/new.ts"]).toBeDefined();
  });

  it("force: re-extracts all files regardless of fingerprints", async () => {
    await reindexRepo(TMP);
    const summary2 = await reindexRepo(TMP, { force: true });
    expect(summary2.forced).toBe(true);
    expect(summary2.added + summary2.updated).toBeGreaterThan(0);
  });

  it("writes traceability.json", async () => {
    await reindexRepo(TMP);
    expect(existsSync(join(TMP, ".index/traceability.json"))).toBe(true);
  });

  it("traceability: auto-detects src/ references in docs", async () => {
    mkdirSync(join(TMP, "docs"), { recursive: true });
    writeFileSync(join(TMP, "docs/design.md"), "See src/index.ts for details.");
    await reindexRepo(TMP);
    const t = JSON.parse(readFileSync(join(TMP, ".index/traceability.json"), "utf-8"));
    expect(t["docs/design.md"]).toContain("src/index.ts");
  });
});
