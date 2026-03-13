import type { SupabaseClient } from "@supabase/supabase-js";
import type { IndexResult } from "@sensei/shared";
import { Scanner } from "./scanner.js";
import { TypeScriptAdapter } from "./adapters/typescript.js";
import { Indexer } from "./indexer.js";

export interface IndexRepoOptions {
  repoPath: string;
  repoId: string;
  client: SupabaseClient;
  include?: string[];
  exclude?: string[];
}

export async function indexRepo(opts: IndexRepoOptions): Promise<IndexResult> {
  const { repoPath, repoId, client } = opts;

  // Load prior scan state for incremental indexing
  let priorState: Array<{ file_path: string; mtime: number; content_hash: string }> = [];
  try {
    const { data } = await client
      .from("scan_state")
      .select("file_path,mtime,content_hash")
      .eq("repo_id", repoId);
    priorState = (data ?? []) as typeof priorState;
  } catch {
    // First run — no prior state
  }

  // Scan
  const scanner = new Scanner({ repoPath, repoId, priorState, include: opts.include, exclude: opts.exclude });
  const scan = await scanner.scan();

  // Parse only changed files
  const adapter = new TypeScriptAdapter();
  const parsedFiles = await Promise.all(
    scan.files
      .filter(f => scan.changed.includes(f.path) && adapter.extensions.some(ext => f.path.endsWith(ext)))
      .map(f => adapter.parse(f).catch(() => null))
  );
  const validParsed = parsedFiles.filter((p): p is NonNullable<typeof p> => p !== null);

  // Index
  const indexer = new Indexer(client);
  return indexer.indexFiles(scan, validParsed);
}
