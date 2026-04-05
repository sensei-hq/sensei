// packages/tools/src/tools/search.e2e.ts
import { describe, it, expect } from "vitest";
import { reindexRepo } from "./reindex.js";
import { search } from "./search.js";
import { isAvailable } from "./embedder.js";
import { loadSenseiConfig, loadCredentials } from "@sensei/shared";
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

    const config = await loadSenseiConfig(SENSEI_ROOT);
    if (!config) {
      console.warn("E2E: no .sensei/config.yaml found — skipping (requires configured DB)");
      return;
    }

    // Check DB reachability before running (supabase may be configured but not started)
    try {
      const res = await fetch(`${config.supabase_url}/rest/v1/`, { signal: AbortSignal.timeout(2000) });
      if (!res.ok && res.status !== 401) throw new Error(`status ${res.status}`);
    } catch {
      console.warn("E2E: Supabase not reachable at", config.supabase_url, "— skipping");
      return;
    }

    // Check that sensei schema is set up (dbd apply may not have run yet)
    const creds = await loadCredentials();
    if (creds) {
      try {
        const res = await fetch(
          `${config.supabase_url}/rest/v1/symbols?limit=1`,
          { headers: { apikey: creds.supabase_service_key, Authorization: `Bearer ${creds.supabase_service_key}` }, signal: AbortSignal.timeout(2000) }
        );
        if (!res.ok) throw new Error(`status ${res.status}`);
      } catch {
        console.warn("E2E: sensei schema not ready (run dbd apply + dbd grants first) — skipping");
        return;
      }
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
