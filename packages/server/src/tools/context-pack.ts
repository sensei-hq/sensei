import type { SupabaseClient } from "@supabase/supabase-js";
import type { ModelBackend } from "@sensei/shared";
import { buildContextPack } from "@sensei/engine";

export async function contextPack(
  client: SupabaseClient,
  backend: ModelBackend,
  repoId: string,
  repoPath: string,
  task: string,
  opts: {
    maxTokens?: number;
    modelId?: string;
    sessionId?: string;
    sessionContext?: string[];
  } = {}
) {
  const pack = await buildContextPack(client, repoId, repoPath, task, { ...opts, backend });
  return {
    id: pack.id,
    task: pack.task,
    totalTokens: pack.totalTokens,
    modelId: pack.modelId ?? null,
    createdAt: pack.createdAt,
    slices: pack.slices.map(s => ({
      kind: s.kind,
      filePath: s.filePath,
      startLine: s.startLine,
      endLine: s.endLine,
      content: s.content,
      tokens: s.tokens,
      score: s.score,
      ...(s.kind === "code" ? { symbolName: s.symbolName } : { heading: s.heading }),
    })),
  };
}
