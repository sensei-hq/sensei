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
          summary: page.summary,
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
    libraryId: string,
    entry: LibEntry,
    pages: DocPage[],
  ): Promise<{ documentsIndexed: number; sectionsIndexed: number }> {
    // 1. Delete existing documents (cascade-deletes sections via FK)
    const { error: deleteError } = await this.db
      .from("documents_in_library")
      .delete()
      .eq("library_id", libraryId);
    if (deleteError) throw new Error(`LibIndexer.indexShared: delete failed: ${deleteError.message}`);

    let documentsIndexed = 0;
    let sectionsIndexed = 0;

    const { splitSections } = await import("./doc-utils.js");

    for (const page of pages) {
      // 2a. Insert document row
      const docRow = {
        library_id: libraryId,
        sequence: page.sequence ?? documentsIndexed,
        title: page.title,
        url: page.url ?? null,
        local_path: page.localPath ?? null,
        summary: page.summary,
        component: page.component ?? null,
        source_type: entry.source_type,
      };

      const { data: docData, error: docErr } = await this.db
        .from("documents_in_library")
        .insert(docRow)
        .select()
        .single();
      if (docErr) throw new Error(`LibIndexer.indexShared: doc insert failed: ${docErr.message}`);

      // docData may be an array or a single object depending on the client
      const docRecord = Array.isArray(docData) ? docData[0] : docData;
      const documentId: string | null = docRecord?.id ?? null;
      documentsIndexed++;

      // 2b. Split content into sections
      const sections = splitSections(page.content);

      if (sections.length > 0) {
        const sectionRows = await Promise.all(
          sections.map(async (section) => {
            const embedding = this.backend
              ? await this.backend.embed(section.content.slice(0, 512))
              : null;
            return {
              library_id: libraryId,
              document_id: documentId,
              sequence: section.sequence,
              title: section.title,
              content: section.content,
              embedding,
            };
          })
        );

        const { error: secErr } = await this.db.from("sections_in_document").insert(sectionRows);
        if (secErr) throw new Error(`LibIndexer.indexShared: section insert failed: ${secErr.message}`);
        sectionsIndexed += sectionRows.length;
      }
    }

    return { documentsIndexed, sectionsIndexed };
  }

  private async embedPage(entry: LibEntry, page: DocPage): Promise<number[] | null> {
    if (!this.backend) return null;
    const input =
      entry.source_type === "llms.txt"
        ? page.summary
        : page.content.slice(0, 512);
    return this.backend.embed(input);
  }
}
