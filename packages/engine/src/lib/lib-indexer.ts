// packages/engine/src/lib/lib-indexer.ts
import type { SupabaseClient } from "@supabase/supabase-js";
import type { LibEntry, DocPage, ModelBackend } from "@sensei/shared";

export class LibIndexer {
  constructor(
    private readonly db: SupabaseClient,
    private readonly backend: ModelBackend | null = null,
  ) {}

  async index(
    repoId: string,
    entry: LibEntry,
    pages: DocPage[],
  ): Promise<{ sectionsIndexed: number }> {
    const { error: deleteError } = await this.db
      .from("lib_doc_sections")
      .delete()
      .eq("repo_id", repoId)
      .eq("lib_name", entry.name);

    if (deleteError) throw new Error(`LibIndexer: delete failed: ${deleteError.message}`);

    const rows = await Promise.all(
      pages.map(async page => {
        const embedding = await this.embedPage(entry, page);
        return {
          repo_id: repoId,
          lib_name: entry.name,
          title: page.title,
          url: page.url ?? null,
          local_path: page.localPath ?? null,
          description: page.description,
          content: page.content ?? null,
          source_type: entry.source_type,
          component: page.component ?? null,
          embedding,
        };
      })
    );

    const { error: insertError } = await this.db.from("lib_doc_sections").insert(rows);
    if (insertError) throw new Error(`LibIndexer: insert failed: ${insertError.message}`);

    return { sectionsIndexed: rows.length };
  }

  async indexShared(
    sharedLibId: string,
    entry: LibEntry,
    pages: DocPage[],
  ): Promise<{ sectionsIndexed: number }> {
    const { error: deleteError } = await this.db
      .from("shared_lib_sections")
      .delete()
      .eq("shared_lib_id", sharedLibId);

    if (deleteError) throw new Error(`LibIndexer.indexShared: delete failed: ${deleteError.message}`);

    const rows = await Promise.all(
      pages.map(async page => {
        const embedding = await this.embedPage(entry, page);
        return {
          shared_lib_id: sharedLibId,
          title: page.title,
          url: page.url ?? null,
          local_path: page.localPath ?? null,
          description: page.description,
          content: page.content ?? null,
          source_type: entry.source_type,
          component: page.component ?? null,
          embedding,
        };
      })
    );

    const { error: insertError } = await this.db.from("shared_lib_sections").insert(rows);
    if (insertError) throw new Error(`LibIndexer.indexShared: insert failed: ${insertError.message}`);

    return { sectionsIndexed: rows.length };
  }

  private async embedPage(entry: LibEntry, page: DocPage): Promise<number[] | null> {
    if (!this.backend) return null;
    const input =
      entry.source_type === "llms.txt"
        ? page.description
        : (page.content ?? page.description).slice(0, 512);
    return this.backend.embed(input);
  }
}
