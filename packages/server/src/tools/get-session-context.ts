// packages/server/src/tools/get-session-context.ts
import type { SupabaseClient } from "@supabase/supabase-js";
import { detectCrashedSessions, type CrashedSession } from "@sensei/engine";
import { getMemoryItems, type MemoryItem } from "@sensei/engine";
import { detectFtrCoaching, type FtrCoachingHint } from "@sensei/engine";
import type { Snapshot } from "@sensei/engine";

export interface SessionContextResult {
  repo_name: string;
  repo_path: string;
  symbol_count: number;
  file_count: number;
  last_indexed_at: string | null;
  stack: string[];
  session_id: string;
  interrupted: Array<{
    sessionId: string;
    crashedAt: string;
    snapshot: Snapshot | null;
  }>;
  memory: {
    decisions: MemoryItem[];
    patterns: MemoryItem[];
    openQuestions: MemoryItem[];
  };
  coaching: FtrCoachingHint[];
  message: string;
}

export async function getSessionContext(
  client: SupabaseClient,
  repoId: string,
  repoPath: string,
  sessionId: string,
): Promise<SessionContextResult> {
  const { data: repo, error } = await client.from("repos").select("*").eq("id", repoId).single();
  if (error || !repo) throw new Error(`Repo not found: ${error?.message ?? "no data"}`);

  const { count: symbolCount } = await client
    .from("symbols")
    .select("*", { count: "exact", head: true })
    .eq("repo_id", repoId);

  const { count: fileCount } = await client
    .from("scan_state")
    .select("*", { count: "exact", head: true })
    .eq("repo_id", repoId);

  const [crashed, allMemory, coaching] = await Promise.all([
    detectCrashedSessions(client, repoId),
    getMemoryItems(client, repoId),
    detectFtrCoaching(client, repoId),
  ]);

  const decisions = allMemory.filter((m: MemoryItem) => m.type === "decision");
  const patterns = allMemory.filter((m: MemoryItem) => m.type === "pattern");
  const openQuestions = allMemory.filter((m: MemoryItem) => m.type === "question" && m.status === "open");

  const interruptedMsg = crashed.length > 0
    ? ` ${crashed.length} interrupted session(s) detected — check interrupted[] for recovery context.`
    : "";

  const coachingMsg = coaching.length > 0
    ? ` ${coaching.length} FTR coaching hint(s) — check coaching[] to improve your score.`
    : "";

  return {
    repo_name: (repo as Record<string, unknown>)?.name as string ?? "unknown",
    repo_path: repoPath,
    symbol_count: symbolCount ?? 0,
    file_count: fileCount ?? 0,
    last_indexed_at: (repo as Record<string, unknown>)?.last_indexed_at as string | null ?? null,
    stack: (repo as Record<string, unknown>)?.stack as string[] ?? [],
    session_id: sessionId,
    interrupted: (crashed as CrashedSession[]).map(c => ({
      sessionId: c.id,
      crashedAt: c.lastHeartbeat,
      snapshot: c.latestSnapshot,
    })),
    memory: { decisions, patterns, openQuestions },
    coaching,
    message: `Repo "${(repo as Record<string, unknown>)?.name as string ?? "unknown"}" — ${symbolCount ?? 0} symbols across ${fileCount ?? 0} files.${interruptedMsg}${coachingMsg} Call search() to find code.`,
  };
}
