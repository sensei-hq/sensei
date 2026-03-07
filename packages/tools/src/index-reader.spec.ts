import { describe, it, expect, beforeEach, afterEach } from "vitest";
import { mkdirSync, writeFileSync, rmSync } from "fs";
import { join } from "path";
import { readLlmSpec, readSymbolMap, readIndexFile } from "./index-reader.js";

const TMP = "/tmp/sensei-test-index-reader";

beforeEach(() => {
  mkdirSync(join(TMP, ".sensei"), { recursive: true });
  writeFileSync(join(TMP, ".sensei/llmspec.yaml"), `
project: test-app
version: 1.0.0
description: A test project
stack: [typescript]
entry_points:
  - path: src/index.ts
    role: server entry
`);
  writeFileSync(join(TMP, ".sensei/symbol-map.json"), JSON.stringify({
    "src/auth.ts": {
      L0: ["login(email: string, password: string): Promise<User>"],
      L1: ["user = login(email, password)\n// returns: Promise<User | null>"],
      L2: ["validate credentials → fetch user → generate token → return user"],
    }
  }));
  writeFileSync(join(TMP, ".sensei/patterns.md"), "# Patterns\n\n- Use repository pattern for DB access");
});

afterEach(() => rmSync(TMP, { recursive: true, force: true }));

describe("readLlmSpec", () => {
  it("reads and parses .llmspec.yaml", async () => {
    const spec = await readLlmSpec(TMP);
    expect(spec.project).toBe("test-app");
    expect(spec.stack).toContain("typescript");
  });

  it("throws if .llmspec.yaml missing", async () => {
    await expect(readLlmSpec("/nonexistent")).rejects.toThrow("No .sensei/llmspec.yaml found");
  });
});

describe("readSymbolMap", () => {
  it("returns symbol map", async () => {
    const map = await readSymbolMap(TMP);
    expect(map["src/auth.ts"].L0).toHaveLength(1);
  });
});

describe("readIndexFile", () => {
  it("reads a named index file", async () => {
    const content = await readIndexFile(TMP, "patterns.md");
    expect(content).toContain("repository pattern");
  });

  it("returns null if file missing", async () => {
    const content = await readIndexFile(TMP, "nonexistent.md");
    expect(content).toBeNull();
  });
});
