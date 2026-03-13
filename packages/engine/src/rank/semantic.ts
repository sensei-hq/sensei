import type { Candidate, RankContext, ScoredCandidate, RankingStrategy } from "./ranking-strategy.js";

export class SemanticStrategy implements RankingStrategy {
  readonly name = "semantic";

  async rank(candidates: Candidate[], ctx: RankContext): Promise<ScoredCandidate[]> {
    const embedding = await ctx.backend.embed(ctx.task);
    if (embedding.length === 0) return [];  // Ollama unavailable — graceful fallback

    const { data, error } = await ctx.db.rpc("match_embeddings", {
      p_repo_id: ctx.repoId,
      query_embedding: embedding,
      match_threshold: 0.3,
      match_count: 20,
    });

    if (error || !data) return [];

    const candidateSet = new Set(candidates.map(c => c.filePath));
    const scoreMap = new Map<string, number>(
      (data as Array<{ file_path: string; similarity: number }>).map(r => [r.file_path, r.similarity])
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
