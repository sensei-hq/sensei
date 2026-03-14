import type { SupabaseClient } from "@supabase/supabase-js";
import type { ModelBackend } from "@sensei/shared";
import { DiffFirstBFSStrategy, BM25Strategy, SemanticStrategy, RankingStrategyChain } from "@sensei/engine";
import type { Candidate } from "@sensei/engine";
import { createTokenCounter } from "@sensei/shared";

const CHANGED_FILES_WINDOW_MS = 24 * 60 * 60 * 1000;

export async function recommendNext(
  client: SupabaseClient,
  backend: ModelBackend,
  repoId: string,
  task: string,
  modelId?: string,
) {
  const counter = createTokenCounter(modelId);

  const { data: allFiles } = await client.from("scan_state").select("file_path").eq("repo_id", repoId);
  const candidates: Candidate[] = (allFiles ?? []).map((f: { file_path: string }) => ({
    filePath: f.file_path,
    type: (f.file_path.endsWith(".md") || f.file_path.endsWith(".mdx")) ? "doc" : "code",
  }));

  const since = new Date(Date.now() - CHANGED_FILES_WINDOW_MS).toISOString();
  const { data: changedData } = await client.from("scan_state").select("file_path").eq("repo_id", repoId).gt("indexed_at", since);
  const changedFiles = (changedData ?? []).map((f: { file_path: string }) => f.file_path);

  const chain = new RankingStrategyChain([new DiffFirstBFSStrategy(), new BM25Strategy(), new SemanticStrategy()]);
  const ranked = await chain.rank(candidates, { task, repoId, changedFiles, db: client, backend, modelId });
  const top3 = ranked.slice(0, 3);

  const recs = await Promise.all(
    top3.map(async candidate => {
      const { data: syms } = await client
        .from("symbols")
        .select("name,signature")
        .eq("repo_id", repoId)
        .eq("file_path", candidate.filePath);

      const estimatedTokens = (syms ?? []).reduce(
        (sum: number, s: { name: string; signature: string | null }) =>
          sum + counter.count(`${s.name} ${s.signature ?? ""}`.trim()),
        0
      );

      return { filePath: candidate.filePath, score: candidate.score, symbolCount: (syms ?? []).length, estimatedTokens };
    })
  );

  return {
    recommendations: recs,
    suggestedBudget: Math.min(Math.ceil(recs.reduce((s, r) => s + r.estimatedTokens, 0) * 1.5), 8000),
  };
}
