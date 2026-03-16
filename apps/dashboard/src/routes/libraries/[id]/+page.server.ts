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
    .from('shared_libs')
    .select('id,name,source_type,base_url,local_path,section_count,indexed_at,index_status,index_error,created_at,icon_url,category,embed_status')
    .eq('id', params.id)
    .single();

  if (!lib) throw error(404, 'Library not found');

  const { data: sections } = await db
    .from('shared_lib_sections')
    .select('id,title,url,description,component,last_fetched')
    .eq('shared_lib_id', params.id)
    .order('title')
    .limit(200);

  const { data: repoLinks } = await db
    .from('repo_libs')
    .select('repos!inner(id,name,local_path)')
    .eq('shared_lib_id', params.id);

  const repos = ((repoLinks ?? []) as Array<{ repos: { id: string; name: string; local_path: string } }>)
    .map(l => l.repos);

  const { data: queries } = await db
    .from('lib_queries')
    .select('id,query_text,source,sections_hit,created_at')
    .eq('shared_lib_id', params.id)
    .order('created_at', { ascending: false })
    .limit(20);

  return {
    lib: lib as { id: string; name: string; source_type: string; base_url: string | null; local_path: string | null; section_count: number; indexed_at: string | null; index_status: string; index_error: string | null; created_at: string; icon_url: string | null; category: Category | null; embed_status: string | null },
    sections: (sections ?? []) as Array<{ id: string; title: string; url: string | null; description: string; component: string | null; last_fetched: string }>,
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
      .from('shared_libs')
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
      .from('repo_libs')
      .update({ source_type, base_url: base_url ?? null, local_path: local_path ?? null })
      .eq('shared_lib_id', params.id);

    const { data: lib } = await db
      .from('shared_libs')
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

    const { data: hits } = await db
      .from('shared_lib_sections')
      .select('id,title,url,description,component')
      .eq('shared_lib_id', params.id)
      .or(`title.ilike.%${query}%,description.ilike.%${query}%`)
      .limit(10);

    const results = hits ?? [];

    await db.from('lib_queries').insert({
      shared_lib_id: params.id,
      query_text: query,
      source: 'simulate',
      sections_hit: results.length,
    });

    return { query, results };
  },

  reindex: async ({ params }) => {
    const db = getDb();
    const { data: lib } = await db
      .from('shared_libs')
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
      .from('shared_libs')
      .select('id,name')
      .eq('id', params.id)
      .single();

    if (!lib) return fail(404, { error: 'Library not found' });

    await startLibEmbed(db, lib.id, lib.name);
    return { embedding: true };
  },
};
