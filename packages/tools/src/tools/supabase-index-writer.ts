// eslint-disable-next-line @typescript-eslint/no-explicit-any
type AnySupabaseClient = any;
import type { SymbolMap } from "@sensei/shared";

export interface TraceabilityEntry {
  docPath: string;
  covers: string[];
  autoDetected: boolean;
}

/** Upsert all symbol rows for a repo. No-op if symbolMap is empty.
 *  Client must be created with db: { schema: "sensei" } (via makeSenseiClient). */
export async function upsertSymbols(
  client: AnySupabaseClient,
  repoId: string,
  symbolMap: SymbolMap,
): Promise<void> {
  const entries = Object.entries(symbolMap);
  if (entries.length === 0) return;

  const rows = entries.map(([file_path, s]) => ({
    repo_id:   repoId,
    file_path,
    l0: s.L0,
    l1: s.L1.join("\n"),
    l2: s.L2.join("\n"),
  }));

  const { error } = await client
    .from("symbol_map")
    .upsert(rows, { onConflict: "repo_id,file_path" });

  if (error) console.error("[indexer] Supabase symbol_map upsert error:", error.message);
}

/** Upsert all doc coverage rows for a repo. No-op if empty.
 *  Client must be created with db: { schema: "sensei" } (via makeSenseiClient). */
export async function upsertDocs(
  client: AnySupabaseClient,
  repoId: string,
  traceability: TraceabilityEntry[],
): Promise<void> {
  if (traceability.length === 0) return;

  const rows = traceability.map(t => ({
    repo_id:       repoId,
    doc_path:      t.docPath,
    covers:        t.covers,
    auto_detected: t.autoDetected,
  }));

  const { error } = await client
    .from("docs")
    .upsert(rows, { onConflict: "repo_id,doc_path" });

  if (error) console.error("[indexer] Supabase docs upsert error:", error.message);
}
