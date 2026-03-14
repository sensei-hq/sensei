// packages/server/src/tools/get-session-context.ts
import type { SupabaseClient } from "@supabase/supabase-js";
import { detectCrashedSessions, type CrashedSession } from "@sensei/engine";
import { getMemoryItems, type MemoryItem } from "@sensei/engine";
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

  const crashed: CrashedSession[] = await detectCrashedSessions(client, repoId);

  const allMemory: MemoryItem[] = await getMemoryItems(client, repoId);
  const decisions = allMemory.filter(m => m.type === "decision");
  const patterns = allMemory.filter(m => m.type === "pattern");
  const openQuestions = allMemory.filter(m => m.type === "question" && m.status === "open");

  const interruptedMsg = crashed.length > 0
    ? ` ${crashed.length} interrupted session(s) detected — check interrupted[] for recovery context.`
    : "";

  return {
    repo_name: repo?.name ?? "unknown",
    repo_path: repoPath,
    symbol_count: symbolCount ?? 0,
    file_count: fileCount ?? 0,
    last_indexed_at: repo?.last_indexed_at ?? null,
    stack: repo?.stack ?? [],
    session_id: sessionId,
    interrupted: crashed.map(c => ({
      sessionId: c.id,
      crashedAt: c.lastHeartbeat,
      snapshot: c.latestSnapshot,
    })),
    memory: { decisions, patterns, openQuestions },
    message: `Repo "${repo?.name ?? "unknown"}" — ${symbolCount ?? 0} symbols across ${fileCount ?? 0} files.${interruptedMsg} Call search() to find code.`,
  };
}
