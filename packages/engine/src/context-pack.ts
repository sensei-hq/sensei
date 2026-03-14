import type { SupabaseClient } from "@supabase/supabase-js";
import type { ContextPack, ModelBackend } from "@sensei/shared";
import { createTokenCounter } from "@sensei/shared";
import type { Candidate } from "./rank/ranking-strategy.js";
import { DiffFirstBFSStrategy } from "./rank/diff-first-bfs.js";
import { BM25Strategy } from "./rank/bm25.js";
import { SemanticStrategy } from "./rank/semantic.js";
import { RankingStrategyChain } from "./rank/chain.js";
import { ASTSlicer } from "./slice/ast-slicer.js";
import { SectionSlicer } from "./slice/section-slicer.js";
import { Assembler } from "./assemble/assembler.js";

const CHANGED_FILES_WINDOW_MS = 24 * 60 * 60 * 1000;

export interface BuildContextPackOptions {
  maxTokens?: number;
  modelId?: string;
  sessionId?: string;
  sessionContext?: string[];
  backend: ModelBackend;
}

export async function buildContextPack(
  db: SupabaseClient,
  repoId: string,
  repoPath: string,
  task: string,
  opts: BuildContextPackOptions,
): Promise<ContextPack> {
  const { maxTokens = 8000, modelId, sessionId, sessionContext, backend } = opts;
  const counter = createTokenCounter(modelId);

  // 1. Load all candidates from scan_state
  const { data: allFiles, error: scanError } = await db
    .from("scan_state")
    .select("file_path")
    .eq("repo_id", repoId);
  if (scanError) throw new Error(`Failed to load scan_state: ${scanError.message}`);

  const candidates: Candidate[] = (allFiles ?? []).map((f: { file_path: string }) => ({
    filePath: f.file_path,
    type: (f.file_path.endsWith(".md") || f.file_path.endsWith(".mdx")) ? "doc" : "code",
  }));

  // 2. Load changed files (last 24h)
  const since = new Date(Date.now() - CHANGED_FILES_WINDOW_MS).toISOString();
  const { data: changedData } = await db
    .from("scan_state")
    .select("file_path")
    .eq("repo_id", repoId)
    .gt("indexed_at", since);
  const changedFiles = (changedData ?? []).map((f: { file_path: string }) => f.file_path);

  // 3. Rank
  const chain = new RankingStrategyChain([
    new DiffFirstBFSStrategy(),
    new BM25Strategy(),
    new SemanticStrategy(),
  ]);
  const ranked = await chain.rank(candidates, { task, repoId, changedFiles, db, backend, modelId });

  // 4. Slice
  const astSlicer = new ASTSlicer(db, repoPath, repoId);
  const sectionSlicer = new SectionSlicer(db, repoId);

  const sliceResults = await Promise.all(
    ranked.map(candidate =>
      candidate.type === "code"
        ? astSlicer.slice(candidate, counter)
        : sectionSlicer.slice(candidate, counter)
    )
  );
  const allSlices = sliceResults.flat();

  // 5. Assemble
  const pack = new Assembler().assemble(allSlices, { maxTokens, counter, task, modelId, sessionContext });

  // 6. Persist
  const { error: upsertError } = await db.from("context_packs").upsert({
    id: pack.id,
    repo_id: repoId,
    session_id: sessionId ?? null,
    task,
    model_id: modelId ?? null,
    slices: pack.slices,
    total_tokens: pack.totalTokens,
    created_at: pack.createdAt,
  });
  if (upsertError) throw new Error(`Failed to persist context pack: ${upsertError.message}`);

  return pack;
}
