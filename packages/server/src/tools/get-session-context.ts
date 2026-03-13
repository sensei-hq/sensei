import type { SupabaseClient } from "@supabase/supabase-js";

export interface SessionContextResult {
  repo_name: string;
  repo_path: string;
  symbol_count: number;
  file_count: number;
  last_indexed_at: string | null;
  stack: string[];
  message: string;
}

export async function getSessionContext(
  client: SupabaseClient,
  repoId: string,
  repoPath: string,
): Promise<SessionContextResult> {
  const { data: repo } = await client.from("repos").select("*").eq("id", repoId).single();
  const { count: symbolCount } = await client
    .from("symbols")
    .select("*", { count: "exact", head: true })
    .eq("repo_id", repoId);
  const { count: fileCount } = await client
    .from("scan_state")
    .select("*", { count: "exact", head: true })
    .eq("repo_id", repoId);

  return {
    repo_name: repo?.name ?? "unknown",
    repo_path: repoPath,
    symbol_count: symbolCount ?? 0,
    file_count: fileCount ?? 0,
    last_indexed_at: repo?.last_indexed_at ?? null,
    stack: repo?.stack ?? [],
    message: `Repo "${repo?.name ?? "unknown"}" — ${symbolCount ?? 0} symbols across ${fileCount ?? 0} files. Call search() to find code.`,
  };
}
