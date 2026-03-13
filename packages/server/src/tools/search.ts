import type { SupabaseClient } from "@supabase/supabase-js";
import type { RepoSymbol } from "@sensei/shared";

export interface SearchResult {
  symbols: Array<{
    name: string;
    kind: string;
    file_path: string;
    line_start: number;
    signature: string | null;
    docstring: string | null;
  }>;
  total: number;
  query: string;
}

export async function search(
  client: SupabaseClient,
  repoId: string,
  query: string,
  limit = 20,
): Promise<SearchResult> {
  // Phase 1: substring text search using ilike. Phase 2 upgrades to BM25 via pg_trgm / to_tsvector.
  const { data, error } = await client
    .from("symbols")
    .select("name,kind,file_path,line_start,signature,docstring")
    .eq("repo_id", repoId)
    .or(`name.ilike.%${query}%,signature.ilike.%${query}%,docstring.ilike.%${query}%`)
    .eq("is_exported", true)
    .order("name")
    .limit(limit);

  if (error) throw new Error(`Search failed: ${error.message}`);

  return {
    symbols: (data ?? []).map(s => ({
      name: s.name,
      kind: s.kind,
      file_path: s.file_path,
      line_start: s.line_start,
      signature: s.signature,
      docstring: s.docstring,
    })),
    total: (data ?? []).length,
    query,
  };
}
