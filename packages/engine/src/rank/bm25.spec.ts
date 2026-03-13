import { describe, it, expect, vi } from "vitest";
import { BM25Strategy } from "./bm25.js";
import type { Candidate, RankContext } from "./ranking-strategy.js";

function makeCtx(rpcResult: Array<{file_path: string; score: number}>): RankContext {
  return {
    task: "fix auth middleware",
    repoId: "repo-1",
    changedFiles: [],
    db: {
      rpc: vi.fn().mockResolvedValue({ data: rpcResult, error: null }),
    } as any,
    backend: { embed: vi.fn().mockResolvedValue([]) } as any,
  };
}

const candidates: Candidate[] = [
  { filePath: "src/auth.ts", type: "code" },
  { filePath: "src/utils.ts", type: "code" },
];

describe("BM25Strategy", () => {
  it("returns scored candidates matching RPC result", async () => {
    const ctx = makeCtx([{ file_path: "src/auth.ts", score: 0.9 }]);
    const strategy = new BM25Strategy();
    const result = await strategy.rank(candidates, ctx);

    expect(result).toHaveLength(1);
    expect(result[0].filePath).toBe("src/auth.ts");
    expect(result[0].score).toBe(0.9);
    expect(result[0].strategyScores["bm25"]).toBe(0.9);
  });

  it("returns [] when RPC returns no matches", async () => {
    const ctx = makeCtx([]);
    const strategy = new BM25Strategy();
    expect(await strategy.rank(candidates, ctx)).toHaveLength(0);
  });

  it("returns [] when RPC errors", async () => {
    const ctx = makeCtx([]);
    (ctx.db.rpc as any).mockResolvedValue({ data: null, error: { message: "query syntax error" } });
    const strategy = new BM25Strategy();
    expect(await strategy.rank(candidates, ctx)).toHaveLength(0);
  });

  it("calls rpc rank_bm25 with correct params", async () => {
    const ctx = makeCtx([]);
    await new BM25Strategy().rank(candidates, ctx);
    expect(ctx.db.rpc).toHaveBeenCalledWith("rank_bm25", { p_repo_id: "repo-1", p_query: "fix auth middleware" });
  });
});
