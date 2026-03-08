import { describe, it, expect, beforeEach, afterEach } from "vitest";
import { writeFile, mkdir, rm } from "fs/promises";
import { join } from "path";
import { tmpdir } from "os";
import { generateCoverage } from "./coverage.js";
import type { ModelBackend } from "@sensei/shared";

function makeModel(responseMap: Record<string, string>): Pick<ModelBackend, "generate"> {
  return {
    async generate(prompt: string) {
      for (const [key, val] of Object.entries(responseMap)) {
        if (prompt.includes(key)) return val;
      }
      return "[]";
    },
  };
}

describe("generateCoverage", () => {
  let repoPath: string;

  beforeEach(async () => {
    repoPath = join(tmpdir(), `coverage-test-${Date.now()}`);
    await mkdir(join(repoPath, ".sensei"), { recursive: true });
    await mkdir(join(repoPath, "docs", "design"), { recursive: true });
  });

  afterEach(async () => {
    await rm(repoPath, { recursive: true, force: true });
  });

  it("returns empty array when symbol-map.json is missing", async () => {
    const model = makeModel({});
    const result = await generateCoverage(repoPath, model);
    expect(result).toEqual([]);
  });

  it("maps a doc to its source files using model response", async () => {
    const symbolMap = {
      "packages/tools/src/tools/reindex.ts": { L0: ["export function reindexRepo"], L1: [], L2: [] },
      "packages/tools/src/tools/query.ts": { L0: ["export function getLlmSpec"], L1: [], L2: [] },
    };
    await writeFile(join(repoPath, ".sensei", "symbol-map.json"), JSON.stringify(symbolMap));
    await writeFile(
      join(repoPath, "docs", "design", "reindex.md"),
      "# Reindex\n\nDescribes the reindexing flow."
    );

    const model = makeModel({
      "docs/design/reindex.md": '["packages/tools/src/tools/reindex.ts"]',
    });

    const result = await generateCoverage(repoPath, model);
    expect(result).toHaveLength(1);
    expect(result[0].path).toBe("docs/design/reindex.md");
    expect(result[0].covers).toEqual(["packages/tools/src/tools/reindex.ts"]);
  });

  it("filters out hallucinated paths not in source files", async () => {
    const symbolMap = {
      "packages/tools/src/tools/reindex.ts": { L0: ["export function reindexRepo"], L1: [], L2: [] },
    };
    await writeFile(join(repoPath, ".sensei", "symbol-map.json"), JSON.stringify(symbolMap));
    await writeFile(join(repoPath, "docs", "design", "foo.md"), "# Foo\n\nSome doc.");

    const model = makeModel({
      "docs/design/foo.md": '["packages/tools/src/tools/reindex.ts", "src/nonexistent.ts"]',
    });

    const result = await generateCoverage(repoPath, model);
    expect(result[0].covers).toEqual(["packages/tools/src/tools/reindex.ts"]);
  });

  it("excludes spec files from source candidates", async () => {
    const symbolMap = {
      "packages/tools/src/tools/reindex.ts": { L0: ["export function reindexRepo"], L1: [], L2: [] },
      "packages/tools/src/tools/reindex.spec.ts": { L0: ["describe('reindexRepo')"], L1: [], L2: [] },
    };
    await writeFile(join(repoPath, ".sensei", "symbol-map.json"), JSON.stringify(symbolMap));
    await writeFile(join(repoPath, "docs", "design", "foo.md"), "# Foo");

    let capturedPrompt = "";
    const model = {
      async generate(prompt: string) {
        capturedPrompt = prompt;
        return "[]";
      },
    };

    await generateCoverage(repoPath, model);
    expect(capturedPrompt).not.toContain("reindex.spec.ts");
  });

  it("handles empty model response gracefully", async () => {
    const symbolMap = {
      "packages/tools/src/tools/query.ts": { L0: ["export function getLlmSpec"], L1: [], L2: [] },
    };
    await writeFile(join(repoPath, ".sensei", "symbol-map.json"), JSON.stringify(symbolMap));
    await writeFile(join(repoPath, "docs", "design", "bar.md"), "# Bar\n\nnothing here");

    const model = makeModel({ "docs/design/bar.md": "I don't know" });
    const result = await generateCoverage(repoPath, model);
    expect(result[0].covers).toEqual([]);
  });
});
