import type { SupabaseClient } from "@supabase/supabase-js";
import { readFile } from "fs/promises";
import { join } from "path";

export interface LoadContextResult {
  file_path: string;
  content: string;
  symbols: Array<{
    name: string;
    kind: string;
    line_start: number;
    line_end: number;
    signature: string | null;
    is_exported: boolean;
  }>;
  line_count: number;
}

export async function loadContext(
  client: SupabaseClient,
  repoId: string,
  repoPath: string,
  filePath: string,
): Promise<LoadContextResult> {
  const absPath = join(repoPath, filePath);
  let content: string;
  try {
    content = await readFile(absPath, "utf-8");
  } catch {
    throw new Error(`File not found: ${filePath}`);
  }

  const { data: symbols, error: symbolsError } = await client
    .from("symbols")
    .select("name,kind,line_start,line_end,signature,is_exported")
    .eq("repo_id", repoId)
    .eq("file_path", filePath)
    .order("line_start");

  if (symbolsError) {
    return {
      file_path: filePath,
      content,
      symbols: [],
      line_count: content.split("\n").length,
      note: `Symbols unavailable: ${symbolsError.message}`,
    } as any;
  }

  return {
    file_path: filePath,
    content,
    symbols: (symbols ?? []).map(s => ({
      name: s.name,
      kind: s.kind,
      line_start: s.line_start,
      line_end: s.line_end,
      signature: s.signature,
      is_exported: s.is_exported,
    })),
    line_count: content.split("\n").length,
  };
}
