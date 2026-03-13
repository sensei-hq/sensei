import { readFile } from "fs/promises";
import { join } from "path";
import type { SupabaseClient } from "@supabase/supabase-js";
import type { CodeSlice, TokenCounter } from "@sensei/shared";
import type { ScoredCandidate } from "../rank/ranking-strategy.js";

export class ASTSlicer {
  constructor(
    private db: SupabaseClient,
    private repoPath: string,
    private repoId: string,
  ) {}

  async slice(candidate: ScoredCandidate, counter: TokenCounter): Promise<CodeSlice[]> {
    const { data: symbols, error } = await this.db
      .from("symbols")
      .select("name,line_start,line_end")
      .eq("repo_id", this.repoId)
      .eq("file_path", candidate.filePath);

    if (error || !symbols || symbols.length === 0) return [];

    let lines: string[];
    try {
      const content = await readFile(join(this.repoPath, candidate.filePath), "utf-8");
      lines = content.split("\n");
    } catch {
      return [];
    }

    return (symbols as Array<{ name: string; line_start: number; line_end: number }>).map(sym => {
      const content = lines.slice(sym.line_start - 1, sym.line_end).join("\n");
      return {
        kind: "code" as const,
        filePath: candidate.filePath,
        startLine: sym.line_start,
        endLine: sym.line_end,
        content,
        tokens: counter.count(content),
        symbolName: sym.name,
        score: candidate.score,
      };
    });
  }
}
