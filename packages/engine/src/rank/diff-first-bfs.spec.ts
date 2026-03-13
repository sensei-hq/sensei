import { describe, it, expect, vi } from "vitest";
import { DiffFirstBFSStrategy } from "./diff-first-bfs.js";
import type { Candidate, RankContext } from "./ranking-strategy.js";

function makeCtx(changedFiles: string[], rpcResult: Array<{file_path: string; score: number}>): RankContext {
  return {
    task: "fix auth",
    repoId: "repo-1",
    changedFiles,
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

describe("DiffFirstBFSStrategy", () => {
  it("returns [] when no changed files", async () => {
    const ctx = makeCtx([], []);
    const strategy = new DiffFirstBFSStrategy();
    const result = await strategy.rank(candidates, ctx);
    expect(result).toHaveLength(0);
  });

  it("returns scored candidates from RPC result", async () => {
    const ctx = makeCtx(["src/auth.ts"], [
      { file_path: "src/auth.ts", score: 2.0 },
      { file_path: "src/utils.ts", score: 1.3 },
    ]);
    const strategy = new DiffFirstBFSStrategy();
    const result = await strategy.rank(candidates, ctx);

    expect(result).toHaveLength(2);
    const auth = result.find(r => r.filePath === "src/auth.ts");
    expect(auth?.score).toBe(2.0);
    expect(auth?.strategyScores["diff_first_bfs"]).toBe(2.0);
  });

  it("only returns candidates present in input list", async () => {
    const ctx = makeCtx(["src/auth.ts"], [
      { file_path: "src/auth.ts", score: 2.0 },
      { file_path: "src/unknown.ts", score: 1.5 }, // not in candidates list
    ]);
    const strategy = new DiffFirstBFSStrategy();
    const result = await strategy.rank(candidates, ctx);

    expect(result.every(r => candidates.some(c => c.filePath === r.filePath))).toBe(true);
  });

  it("calls rpc rank_bfs with correct params", async () => {
    const ctx = makeCtx(["src/auth.ts"], []);
    const strategy = new DiffFirstBFSStrategy();
    await strategy.rank(candidates, ctx);

    expect(ctx.db.rpc).toHaveBeenCalledWith("rank_bfs", {
      p_repo_id: "repo-1",
      p_changed_files: ["src/auth.ts"],
    });
  });
});
