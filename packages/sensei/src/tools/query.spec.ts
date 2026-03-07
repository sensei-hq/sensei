import { describe, it, expect, beforeEach, afterEach } from "vitest";
import { mkdirSync, writeFileSync, rmSync } from "fs";
import { join } from "path";
import { getLlmSpec, getFileContext, listExports, findPattern, getShortcuts } from "./query.js";

const TMP = "/tmp/sensei-test-query";

beforeEach(() => {
  mkdirSync(join(TMP, ".index"), { recursive: true });
  mkdirSync(join(TMP, "src"), { recursive: true });
  writeFileSync(join(TMP, ".index/llmspec.yaml"), `
project: query-test
stack: [typescript]
description: test
shortcuts:
  dev: bun run dev
  test: bun test
`);
  writeFileSync(join(TMP, ".index/symbol-map.json"), JSON.stringify({
    "src/auth.ts": {
      L0: ["login(email: string): Promise<User>"],
      L1: ["user = login(email)\n// returns: Promise<User>"],
      L2: ["validate → fetch → return"],
    }
  }));
  writeFileSync(join(TMP, ".index/patterns.md"), "# Patterns\n\n## Repository Pattern\nAll DB access through repos.");
  writeFileSync(join(TMP, ".index/shortcuts.md"), "# Shortcuts\n\n- dev: bun run dev\n- test: bun test");
  writeFileSync(join(TMP, "src/auth.ts"), `export function login(email: string): Promise<User> {\n  return db.find(email);\n}`);
});

afterEach(() => rmSync(TMP, { recursive: true, force: true }));

describe("getLlmSpec", () => {
  it("returns full spec by default", async () => {
    const result = await getLlmSpec(TMP);
    expect(result).toContain("query-test");
  });

  it("returns named section only", async () => {
    const result = await getLlmSpec(TMP, "shortcuts");
    expect(result).toContain("bun run dev");
    expect(result).not.toContain("query-test");
  });
});

describe("getFileContext", () => {
  it("returns L0 signatures", async () => {
    const result = await getFileContext(TMP, "src/auth.ts", "L0");
    expect(result).toContain("login(email: string): Promise<User>");
    expect(result).not.toContain("db.find");
  });

  it("returns L3 full source", async () => {
    const result = await getFileContext(TMP, "src/auth.ts", "L3");
    expect(result).toContain("db.find(email)");
  });
});

describe("listExports", () => {
  it("lists all exports at L0", async () => {
    const result = await listExports(TMP);
    expect(result).toContain("src/auth.ts");
    expect(result).toContain("login(email: string): Promise<User>");
  });

  it("scopes to module path", async () => {
    const result = await listExports(TMP, "src/auth");
    expect(result).toContain("login");
  });
});

describe("findPattern", () => {
  it("returns named pattern content", async () => {
    const result = await findPattern(TMP, "Repository");
    expect(result).toContain("DB access through repos");
  });

  it("returns all patterns if no name given", async () => {
    const result = await findPattern(TMP);
    expect(result).toContain("Repository Pattern");
  });
});

describe("getShortcuts", () => {
  it("returns shortcuts", async () => {
    const result = await getShortcuts(TMP);
    expect(result).toContain("bun run dev");
  });
});
