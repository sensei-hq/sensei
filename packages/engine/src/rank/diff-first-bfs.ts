import type { Candidate, RankContext, ScoredCandidate, RankingStrategy } from "./ranking-strategy.js";

export class DiffFirstBFSStrategy implements RankingStrategy {
  readonly name = "diff_first_bfs";

  async rank(candidates: Candidate[], ctx: RankContext): Promise<ScoredCandidate[]> {
    if (ctx.changedFiles.length === 0) return [];

    const { data, error } = await ctx.db.rpc("rank_bfs", {
      p_repo_id: ctx.repoId,
      p_changed_files: ctx.changedFiles,
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
