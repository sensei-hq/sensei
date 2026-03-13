import { describe, it, expect, vi } from "vitest";
import { RankingStrategyChain } from "./chain.js";
import type { Candidate, RankContext, RankingStrategy } from "./ranking-strategy.js";

function makeStrategy(name: string, scores: Record<string, number>): RankingStrategy {
  return {
    name,
    rank: vi.fn(async (candidates: Candidate[]) =>
      candidates
        .filter(c => scores[c.filePath] !== undefined)
        .map(c => ({
          ...c,
          score: scores[c.filePath],
          strategyScores: { [name]: scores[c.filePath] },
        }))
    ),
  };
}

const ctx: RankContext = {
  task: "fix auth",
  repoId: "repo-1",
  changedFiles: [],
  db: {} as any,
  backend: {} as any,
};

const candidates: Candidate[] = [
  { filePath: "src/auth.ts", type: "code" },
  { filePath: "src/utils.ts", type: "code" },
  { filePath: "src/db.ts", type: "code" },
];

describe("RankingStrategyChain", () => {
  it("merges scores using max per file", async () => {
    const s1 = makeStrategy("s1", { "src/auth.ts": 2.0, "src/utils.ts": 0.5 });
    const s2 = makeStrategy("s2", { "src/auth.ts": 0.8, "src/db.ts": 1.5 });
    const chain = new RankingStrategyChain([s1, s2]);
    const result = await chain.rank(candidates, ctx);

    expect(result.find(r => r.filePath === "src/auth.ts")?.score).toBe(2.0);  // max(2.0, 0.8)
    expect(result.find(r => r.filePath === "src/db.ts")?.score).toBe(1.5);
    expect(result.find(r => r.filePath === "src/utils.ts")?.score).toBe(0.5);
  });

  it("returns results sorted descending by score", async () => {
    const s1 = makeStrategy("s1", { "src/auth.ts": 1.0, "src/utils.ts": 0.5, "src/db.ts": 1.5 });
    const chain = new RankingStrategyChain([s1]);
    const result = await chain.rank(candidates, ctx);

    expect(result[0].filePath).toBe("src/db.ts");
    expect(result[1].filePath).toBe("src/auth.ts");
    expect(result[2].filePath).toBe("src/utils.ts");
  });

  it("limits output to top 20", async () => {
    const manyCandidates: Candidate[] = Array.from({ length: 30 }, (_, i) => ({
      filePath: `src/file${i}.ts`,
      type: "code" as const,
    }));
    const scores = Object.fromEntries(manyCandidates.map((c, i) => [c.filePath, i * 0.1]));
    const chain = new RankingStrategyChain([makeStrategy("s1", scores)]);
    const result = await chain.rank(manyCandidates, ctx);
    expect(result.length).toBeLessThanOrEqual(20);
  });

  it("runs all strategies and calls each rank() once", async () => {
    const s1 = makeStrategy("s1", {});
    const s2 = makeStrategy("s2", {});
    await new RankingStrategyChain([s1, s2]).rank(candidates, ctx);
    expect(s1.rank).toHaveBeenCalledOnce();
    expect(s2.rank).toHaveBeenCalledOnce();
  });

  it("merges strategyScores from all strategies", async () => {
    const s1 = makeStrategy("s1", { "src/auth.ts": 2.0 });
    const s2 = makeStrategy("s2", { "src/auth.ts": 0.8 });
    const chain = new RankingStrategyChain([s1, s2]);
    const result = await chain.rank(candidates, ctx);
    const auth = result.find(r => r.filePath === "src/auth.ts");
    expect(auth?.strategyScores["s1"]).toBe(2.0);
    expect(auth?.strategyScores["s2"]).toBe(0.8);
  });
});
