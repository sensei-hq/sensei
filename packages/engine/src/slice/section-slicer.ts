import type { SupabaseClient } from "@supabase/supabase-js";
import type { DocSlice, TokenCounter } from "@sensei/shared";
import type { ScoredCandidate } from "../rank/ranking-strategy.js";

export class SectionSlicer {
  constructor(
    private db: SupabaseClient,
    private repoId: string,
  ) {}

  async slice(candidate: ScoredCandidate, counter: TokenCounter): Promise<DocSlice[]> {
    const { data: sections, error } = await this.db
      .from("doc_sections")
      .select("heading,level,start_line,end_line,content")
      .eq("repo_id", this.repoId)
      .eq("file_path", candidate.filePath);

    if (error || !sections || sections.length === 0) return [];

    return (sections as Array<{
      heading: string; level: number;
      start_line: number; end_line: number; content: string;
    }>).map(s => ({
      kind: "doc" as const,
      filePath: candidate.filePath,
      heading: s.heading,
      startLine: s.start_line,
      endLine: s.end_line,
      content: s.content,
      tokens: counter.count(s.content),
      score: candidate.score,
    }));
  }
}
