import type { PageServerLoad, Actions } from './$types';
import { error, fail } from '@sveltejs/kit';
import { getDb } from '$lib/server/db';
import { inferSourceType } from '@sensei/engine';
import { startLibFetch, startLibEmbed, type LibInfo } from '$lib/server/lib-indexer';

const CATEGORIES = ['ui','auth','api','data','test','build','other'] as const;
type Category = typeof CATEGORIES[number];

export const load: PageServerLoad = async ({ params }) => {
  const db = getDb();

  const { data: lib } = await db
    .from('libraries')
    .select('id,name,source_type,base_url,local_path,section_count,document_count,indexed_at,index_status,index_error,created_at,icon_url,category,embed_status')
    .eq('id', params.id)
    .single();

  if (!lib) throw error(404, 'Library not found');

  const { data: documents } = await db
    .from('documents_in_library')
    .select('id,title,url,local_path,summary,component,source_type,sequence,last_fetched')
    .eq('library_id', params.id)
    .order('sequence')
    .limit(200);

  const { data: repoLinks } = await db
    .from('referenced_libraries')
    .select('repos!inner(id,name,local_path)')
    .eq('library_id', params.id);

  const repos = ((repoLinks ?? []) as unknown as Array<{ repos: { id: string; name: string; local_path: string } }>)
    .map(l => l.repos);

  const { data: queries } = await db
    .from('queries_on_library')
    .select('id,query_text,source,sections_hit,created_at')
    .eq('library_id', params.id)
    .order('created_at', { ascending: false })
    .limit(20);

  return {
    lib: lib as { id: string; name: string; source_type: string; base_url: string | null; local_path: string | null; section_count: number; document_count: number; indexed_at: string | null; index_status: string; index_error: string | null; created_at: string; icon_url: string | null; category: Category | null; embed_status: string | null },
    documents: (documents ?? []) as Array<{ id: string; title: string; url: string | null; local_path: string | null; summary: string | null; component: string | null; source_type: string | null; sequence: number; last_fetched: string | null }>,
    repos,
    queries: (queries ?? []) as Array<{ id: string; query_text: string; source: string; sections_hit: number; created_at: string }>,
  };
};

export const actions: Actions = {
  edit: async ({ params, request }) => {
    const db = getDb();
    const formData = await request.formData();
    const url = String(formData.get('url') ?? '').trim();
    const iconUrl = String(formData.get('icon_url') ?? '').trim() || null;
    const category = String(formData.get('category') ?? '').trim() || null;

    if (!url) return fail(400, { error: 'URL or path is required' });
    if (url.startsWith('http://') || url.startsWith('https://')) {
      try { new URL(url); } catch { return fail(400, { error: 'Invalid URL' }); }
    }
    if (category && !CATEGORIES.includes(category as Category)) {
      return fail(400, { error: 'Invalid category' });
    }

    const inferred = inferSourceType(url);
    const { source_type } = inferred;
    const base_url = 'base_url' in inferred ? (inferred.base_url ?? null) : null;
    const local_path = 'local_path' in inferred ? (inferred.local_path ?? null) : null;

    const { error: updateErr } = await db
      .from('libraries')
      .update({
        source_type,
        base_url: base_url ?? null,
        local_path: local_path ?? null,
        icon_url: iconUrl,
        category: category as Category | null,
        index_status: 'pending',
        index_error: null,
      })
      .eq('id', params.id);

    if (updateErr) return fail(500, { error: updateErr.message });

    await db
      .from('referenced_libraries')
      .update({ source_type, base_url: base_url ?? null, local_path: local_path ?? null })
      .eq('library_id', params.id);

    const { data: lib } = await db
      .from('libraries')
      .select('id,name,source_type,base_url,local_path')
      .eq('id', params.id)
      .single();

    if (lib) {
      await startLibFetch(db, lib as LibInfo);
    }

    return { edited: true };
  },

  simulate: async ({ params, request }) => {
    const db = getDb();
    const formData = await request.formData();
    const query = String(formData.get('query') ?? '').trim();
    if (!query) return fail(400, { error: 'Query is required' });

    type SimResult = { id: string; title: string; content: string; document: unknown; similarity?: number };
    let results: SimResult[] = [];

    // Try vector similarity search first (requires embeddings to have been built)
    try {
      const { TransformersBackend } = await import('@sensei/engine');
      const backend = new TransformersBackend();
      await backend.init();
      const embedding = await backend.embed(query);

      const { data: rpcData } = await db.rpc('match_libraries_sections', {
        p_library_id: params.id,
        p_component: null,
        query_embedding: embedding,
        match_count: 10,
      });

      if (rpcData && rpcData.length > 0) {
        results = (rpcData as any[]).map(r => ({
          id: r.section_id,
          title: r.section_title,
          content: r.content,
          similarity: r.similarity,
          document: { title: r.doc_title, url: r.url, component: r.component, summary: r.summary },
        }));
      }
    } catch (err) {
      console.warn('[simulate] Vector search failed:', err instanceof Error ? err.message : String(err));
    }

    // Keyword fallback — used when no embeddings built yet, or vector returned nothing
    if (results.length === 0) {
      const safeQuery = query.replace(/[%,()]/g, '');
      const { data: hits } = await db
        .from('sections_in_document')
        .select('id,title,content,document_id,documents_in_library!inner(title,url,component,summary)')
        .eq('library_id', params.id)
        .or(`title.ilike.%${safeQuery}%,content.ilike.%${safeQuery}%`)
        .limit(10);

      results = (hits ?? []).map((h: any) => ({
        id: h.id,
        title: h.title,
        content: h.content,
        document: h.documents_in_library,
      }));
    }

    await db.from('queries_on_library').insert({
      library_id: params.id,
      query_text: query,
      source: 'simulate',
      sections_hit: results.length,
    });

    return { query, results };
  },

  reindex: async ({ params }) => {
    const db = getDb();
    const { data: lib } = await db
      .from('libraries')
      .select('id,name,source_type,base_url,local_path')
      .eq('id', params.id)
      .single();

    if (!lib) return fail(404, { error: 'Library not found' });

    await startLibFetch(db, lib as { id: string; name: string; source_type: string; base_url: string | null; local_path: string | null });
    return { reindexing: true };
  },

  embed: async ({ params }) => {
    const db = getDb();
    const { data: lib } = await db
      .from('libraries')
      .select('id,name,embed_status')
      .eq('id', params.id)
      .single();

    if (!lib) return fail(404, { error: 'Library not found' });
    if (lib.embed_status === 'embedding') return fail(409, { error: 'Embedding already in progress' });

    await startLibEmbed(db, lib.id, lib.name);
    return { embedding: true };
  },
};
