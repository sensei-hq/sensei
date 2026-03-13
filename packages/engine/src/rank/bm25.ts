import type { Candidate, RankContext, ScoredCandidate, RankingStrategy } from "./ranking-strategy.js";

export class BM25Strategy implements RankingStrategy {
  readonly name = "bm25";

  async rank(candidates: Candidate[], ctx: RankContext): Promise<ScoredCandidate[]> {
    const { data, error } = await ctx.db.rpc("rank_bm25", {
      p_repo_id: ctx.repoId,
      p_query: ctx.task,
    });

    if (error || !data) return [];

    const candidateSet = new Set(candidates.map(c => c.filePath));
    const scoreMap = new Map<string, number>(
      (data as Array<{ file_path: string; score: number }>).map(r => [r.file_path, r.score])
    );

    return candidates
      .filter(c => scoreMap.has(c.filePath) && candidateSet.has(c.filePath))
      .map(c => ({
        ...c,
        score: scoreMap.get(c.filePath)!,
        strategyScores: { [this.name]: scoreMap.get(c.filePath)! },
      }));
  }
}
