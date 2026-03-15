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
    let rows: Record<string, unknown>[];

    if (opts?.query) {
      const embedding = await backend.embed(opts.query);
      const { data, error } = await db.rpc("match_lib_doc_sections", {
        p_repo_id: repoId,
        p_lib_name: lib,
        p_component: opts?.component ?? null,
        query_embedding: embedding,
        match_count: limit,
      });
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

    const sections: DocPage[] = rows.map(r => ({
      title: r.title as string,
      url: (r.url as string | null) ?? undefined,
      localPath: (r.local_path as string | null) ?? undefined,
      description: r.description as string,
      content: (r.content as string | null) ?? undefined,
      sourceType: r.source_type as DocPage["sourceType"],
      component: (r.component as string | null) ?? undefined,
    }));

    return { lib, sections };
  } catch {
    return { lib, sections: [] };
  }
}
