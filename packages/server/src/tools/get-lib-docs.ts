// packages/server/src/tools/get-lib-docs.ts
import type { SupabaseClient } from "@supabase/supabase-js";
import type { ModelBackend } from "@sensei/shared";

export interface LibSection {
  title: string;        // section title (H2 heading)
  content: string;      // section markdown
  document: {
    title: string;
    url: string | null;
    component: string | null;
    summary: string;
  };
  similarity?: number;
}

export async function getLibDocsTool(
  db: SupabaseClient,
  backend: ModelBackend,
  repoId: string,
  lib: string,
  opts?: { component?: string; query?: string; limit?: number },
): Promise<{ lib: string; sections: LibSection[] }> {
  const limit = opts?.limit ?? 10;
  try {
    // Determine if this lib links to the shared pool.
    // The typeof guard preserves backward compatibility with test mocks that only have
    // `rpc` and `from` (no `.schema`) — those return null here and fall through to the
    // existing per-repo path.
    const { data: repoLib } = typeof (db as any).schema === 'function'
      ? await (db as any)
          .schema('sensei')
          .from('repo_libs')
          .select('shared_lib_id')
          .eq('repo_id', repoId)
          .eq('name', lib)
          .maybeSingle()
      : { data: null };

    const sharedLibId: string | null = repoLib?.shared_lib_id ?? null;
    let rows: Record<string, unknown>[];

    // Note: the null-check on db.schema preserves backward compatibility with existing tests
    // that mock only `rpc` and `from` without `.schema` (those mocks return shared_lib_id: null
    // and fall through to the existing per-repo path).
    if (opts?.query) {
      const embedding = await backend.embed(opts.query);
      if (sharedLibId) {
        const { data, error } = await db.rpc("match_libraries_sections", {
          p_library_id: sharedLibId,
          p_component: opts.component ?? null,
          query_embedding: embedding,
          match_count: limit,
        });
        if (error) throw new Error(error.message);
        rows = (data ?? []) as Record<string, unknown>[];
        // Fallback to keyword search if no embeddings exist yet
        if (rows.length === 0) {
          const safeQ = opts.query.replace(/[%,()]/g, '');
          const { data: kData, error: kErr } = await (db as any)
            .schema('sensei')
            .from('sections_in_document')
            .select('id,title,content,document_id,documents_in_library!inner(title,url,local_path,component,summary)')
            .eq('library_id', sharedLibId)
            .or(`title.ilike.%${safeQ}%,content.ilike.%${safeQ}%`)
            .order('title')
            .limit(limit);
          if (kErr) throw new Error(kErr.message);
          rows = (kData ?? []) as Record<string, unknown>[];
        }
      } else {
        const { data, error } = await db.rpc("match_lib_doc_sections", {
          p_repo_id: repoId,
          p_lib_name: lib,
          p_component: opts?.component ?? null,
          query_embedding: embedding,
          match_count: limit,
        });
        if (error) throw new Error(error.message);
        rows = (data ?? []) as Record<string, unknown>[];
      }
    } else {
      // Browse path (no query): list all sections sorted by title
      if (sharedLibId) {
        const { data, error } = await (db as any)
          .schema('sensei')
          .from('sections_in_document')
          .select('id,title,content,documents_in_library!inner(title,url,local_path,component,summary)')
          .eq('library_id', sharedLibId)
          .order('title');
        if (error) throw new Error(error.message);
        rows = (data ?? []) as Record<string, unknown>[];
      } else {
        let query = db
          .from("lib_doc_sections")
          .select("title,url,local_path,description,content,source_type,component")
          .eq("repo_id", repoId)
          .eq("lib_name", lib);
        if (opts?.component) query = query.eq("component", opts.component) as typeof query;
        const { data, error } = await query.order("title");
        if (error) throw new Error(error.message);
        rows = (data ?? []) as Record<string, unknown>[];
      }
    }

    const sections: LibSection[] = rows.map(r => {
      // Handle both RPC shape and JOIN shape
      const doc = (r.documents_in_library as any) ?? {
        title: r.doc_title as string,
        url: r.url as string | null,
        component: r.component as string | null,
        summary: r.summary as string,
      };
      return {
        title: (r.section_title ?? r.title) as string,
        content: r.content as string,
        document: {
          title: doc.title,
          url: doc.url ?? doc.local_path ?? null,
          component: doc.component ?? null,
          summary: doc.summary ?? "",
        },
        similarity: r.similarity as number | undefined,
      };
    });

    // Log MCP query (fire-and-forget, non-blocking)
    if (opts?.query && sharedLibId && rows.length >= 0) {
      try {
        (db as any)
          .schema('sensei')
          .from('queries_on_library')
          .insert({ library_id: sharedLibId, query_text: opts.query, source: 'mcp', sections_hit: rows.length })
          .then(() => {/* ignore */})
          .catch(() => {/* ignore */});
      } catch { /* ignore */ }
    }

    return { lib, sections };
  } catch (err) {
    // Log warning before returning empty sections (spec: "log warning" on failure)
    console.warn(`getLibDocsTool: error fetching sections for lib "${lib}":`, err instanceof Error ? err.message : String(err));
    return { lib, sections: [] };
  }
}
