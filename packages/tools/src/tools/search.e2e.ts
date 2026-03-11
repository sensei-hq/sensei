// packages/tools/src/tools/search.e2e.ts
import { describe, it, expect } from "vitest";
import { reindexRepo } from "./reindex.js";
import { search } from "./search.js";
import { isAvailable } from "./embedder.js";
import { join } from "path";
import { existsSync } from "fs";

// Auto-detect sensei repo root (two levels up from packages/tools/src/tools/)
const SENSEI_ROOT = join(import.meta.dirname, "../../../../");

describe("search e2e", () => {
  it("reindex + search returns reindex.ts for 'reindex repository' query", async () => {
    if (!existsSync(join(SENSEI_ROOT, ".git"))) {
      console.warn("E2E: not in a git repo, skipping");
      return;
    }

    await reindexRepo(SENSEI_ROOT);

    const results = await search(SENSEI_ROOT, "reindex repository");
    expect(typeof results).not.toBe("string");
    const arr = results as Awaited<ReturnType<typeof search>> & Array<unknown>;
    expect(arr.length).toBeGreaterThan(0);

    const hasReindexFile = (arr as Array<{ file: string }>).slice(0, 3).some(r =>
      r.file.includes("reindex")
    );
    expect(hasReindexFile).toBe(true);

    const modelAvail = await isAvailable();
    const firstResult = arr[0] as { matchedBy: string[] };
    const hasBm25OrSymbol = firstResult.matchedBy.includes("bm25") || firstResult.matchedBy.includes("symbol");
    expect(hasBm25OrSymbol).toBe(true);

    if (modelAvail) {
      console.log("Semantic search was available — full e2e ran.");
    } else {
      console.log("Semantic unavailable — symbol + BM25 layers verified only.");
    }
  }, 120_000); // 2-minute timeout for model download
});
