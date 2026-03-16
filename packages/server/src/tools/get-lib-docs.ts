// packages/server/src/tools/get-lib-docs.ts
import type { SupabaseClient } from "@supabase/supabase-js";
import type { ModelBackend, DocPage } from "@sensei/shared";

export async function getLibDocsTool(
  db: SupabaseClient,
  backend: ModelBackend,
  repoId: string,
  lib: string,
  opts?: { component?: string; query?: string; limit?: number },
): Promise<{ lib: string; sections: DocPage[] }> {
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
        const { data, error } = await db.rpc("match_shared_lib_sections", {
          p_shared_lib_id: sharedLibId,
          p_component: opts.component ?? null,
          query_embedding: embedding,
          match_count: limit,
        });
        if (error) throw new Error(error.message);
        rows = (data ?? []) as Record<string, unknown>[];
        // Fallback to keyword search if no embeddings exist yet
        if (rows.length === 0) {
          const q = opts.query;
          let kq = (db as any)
            .schema('sensei')
            .from('shared_lib_sections')
            .select('title,url,local_path,description,content,source_type,component')
            .eq('shared_lib_id', sharedLibId)
            .or(`title.ilike.%${q}%,description.ilike.%${q}%`);
          if (opts.component) kq = kq.eq('component', opts.component);
          const { data: kData, error: kErr } = await kq.order('title').limit(limit);
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
        let q = (db as any)
          .schema('sensei')
          .from('shared_lib_sections')
          .select('title,url,local_path,description,content,source_type,component')
          .eq('shared_lib_id', sharedLibId);
        if (opts?.component) q = q.eq('component', opts.component) as typeof q;
        const { data, error } = await q.order('title');
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

    const sections: DocPage[] = rows.map(r => ({
      title: r.title as string,
      url: (r.url as string | null) ?? undefined,
      localPath: (r.local_path as string | null) ?? undefined,
      description: r.description as string,
      content: (r.content as string | null) ?? undefined,
      sourceType: r.source_type as DocPage["sourceType"],
      component: (r.component as string | null) ?? undefined,
    }));

    // Log MCP query (fire-and-forget, non-blocking)
    if (opts?.query && sharedLibId && rows.length >= 0) {
      try {
        (db as any)
          .schema('sensei')
          .from('lib_queries')
          .insert({ shared_lib_id: sharedLibId, query_text: opts.query, source: 'mcp', sections_hit: rows.length })
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
