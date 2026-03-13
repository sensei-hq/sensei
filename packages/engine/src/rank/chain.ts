import type { Candidate, RankContext, ScoredCandidate, RankingStrategy } from "./ranking-strategy.js";

export class RankingStrategyChain {
  constructor(private strategies: RankingStrategy[]) {}

  async rank(candidates: Candidate[], ctx: RankContext): Promise<ScoredCandidate[]> {
    const allResults = await Promise.all(
      this.strategies.map(s => s.rank(candidates, ctx).catch(() => [] as ScoredCandidate[]))
    );

    // Merge: max score per file, union strategyScores
    const merged = new Map<string, ScoredCandidate>();
    for (const results of allResults) {
      for (const candidate of results) {
        const existing = merged.get(candidate.filePath);
        if (!existing) {
          merged.set(candidate.filePath, { ...candidate });
        } else {
          existing.score = Math.max(existing.score, candidate.score);
          Object.assign(existing.strategyScores, candidate.strategyScores);
        }
      }
    }

    return Array.from(merged.values())
      .sort((a, b) => b.score - a.score)
      .slice(0, 20);
  }
}
