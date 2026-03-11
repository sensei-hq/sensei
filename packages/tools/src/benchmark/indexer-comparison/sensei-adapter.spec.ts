import { describe, it, expect, beforeEach, afterEach } from "vitest";
import { loadSenseiIndex } from "./sensei-adapter.js";
import { writeFile, mkdir, rm } from "fs/promises";
import { join } from "path";
import { tmpdir } from "os";

describe("loadSenseiIndex", () => {
  let dir: string;

  beforeEach(async () => {
    dir = join(tmpdir(), `sensei-test-${Date.now()}`);
    await mkdir(join(dir, ".sensei"), { recursive: true });
  });

  afterEach(() => rm(dir, { recursive: true, force: true }));

  it("returns symbols from symbol-map.json", async () => {
    const symbolMap = {
      "packages/tools/src/index.ts": {
        L0: ["export function reindexRepo"],
        L1: ["Rebuilds the index for a repository."],
      }
    };
    await writeFile(join(dir, ".sensei", "symbol-map.json"), JSON.stringify(symbolMap));
    const result = await loadSenseiIndex(dir);
    expect(result.symbols).toHaveLength(1);
    expect(result.symbols[0].name).toBe("reindexRepo");
    expect(result.files).toContain("packages/tools/src/index.ts");
  });

  it("returns empty result when symbol-map missing", async () => {
    const result = await loadSenseiIndex(dir);
    expect(result.symbols).toHaveLength(0);
    expect(result.missing).toBe(true);
  });
});
