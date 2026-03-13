import { describe, it, expect, vi } from "vitest";
import { SemanticStrategy } from "./semantic.js";
import type { Candidate, RankContext } from "./ranking-strategy.js";

function makeCtx(
  embedding: number[],
  rpcResult: Array<{file_path: string; similarity: number}>
): RankContext {
  return {
    task: "fix auth middleware",
    repoId: "repo-1",
    changedFiles: [],
    db: {
      rpc: vi.fn().mockResolvedValue({ data: rpcResult, error: null }),
    } as any,
    backend: { embed: vi.fn().mockResolvedValue(embedding) } as any,
  };
}

const candidates: Candidate[] = [
  { filePath: "src/auth.ts", type: "code" },
  { filePath: "src/utils.ts", type: "code" },
];

describe("SemanticStrategy", () => {
  it("returns [] when backend.embed returns empty array (Ollama unavailable)", async () => {
    const ctx = makeCtx([], []);
    expect(await new SemanticStrategy().rank(candidates, ctx)).toHaveLength(0);
  });

  it("returns scored candidates from RPC when embedding available", async () => {
    const embedding = new Array(768).fill(0.1);
    const ctx = makeCtx(embedding, [{ file_path: "src/auth.ts", similarity: 0.85 }]);
    const result = await new SemanticStrategy().rank(candidates, ctx);

    expect(result).toHaveLength(1);
    expect(result[0].filePath).toBe("src/auth.ts");
    expect(result[0].score).toBe(0.85);
    expect(result[0].strategyScores["semantic"]).toBe(0.85);
  });

  it("calls backend.embed with the task string", async () => {
    const embedding = new Array(768).fill(0.1);
    const ctx = makeCtx(embedding, []);
    await new SemanticStrategy().rank(candidates, ctx);
    expect(ctx.backend.embed).toHaveBeenCalledWith("fix auth middleware");
  });

  it("calls rpc match_embeddings with correct params", async () => {
    const embedding = new Array(768).fill(0.1);
    const ctx = makeCtx(embedding, []);
    await new SemanticStrategy().rank(candidates, ctx);
    expect(ctx.db.rpc).toHaveBeenCalledWith("match_embeddings", {
      p_repo_id: "repo-1",
      query_embedding: embedding,
      match_threshold: 0.3,
      match_count: 20,
    });
  });
});
