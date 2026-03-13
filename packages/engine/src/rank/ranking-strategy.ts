import type { SupabaseClient } from "@supabase/supabase-js";
import type { ModelBackend } from "@sensei/shared";

export interface Candidate {
  filePath: string;
  type: "code" | "doc";
}

export interface RankContext {
  task: string;
  repoId: string;
  changedFiles: string[];     // repo-relative paths changed in last 24h
  db: SupabaseClient;
  backend: ModelBackend;      // injected — used by SemanticStrategy for embed()
  modelId?: string;
}

export interface ScoredCandidate extends Candidate {
  score: number;
  strategyScores: Record<string, number>;
}

export interface RankingStrategy {
  readonly name: string;
  rank(candidates: Candidate[], ctx: RankContext): Promise<ScoredCandidate[]>;
}
