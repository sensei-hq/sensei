import type { PageServerLoad, Actions } from './$types';
import { fail } from '@sveltejs/kit';
import { getDb } from '$lib/server/db';
import { inferSourceType } from '@sensei/engine';
import { startLibFetch } from '$lib/server/lib-indexer';

export const load: PageServerLoad = async () => {
  const db = getDb();

  const { data: libs } = await db
    .from('shared_libs')
    .select('id,name,source_type,base_url,local_path,section_count,indexed_at,index_status,index_error')
    .order('name');

  const { data: repoLinks } = await db
    .from('repo_libs')
    .select('shared_lib_id,repos!inner(id,name)')
    .not('shared_lib_id', 'is', null);

  const repoMap = new Map<string, Array<{ id: string; name: string }>>();
  for (const link of (repoLinks ?? []) as Array<{ shared_lib_id: string; repos: { id: string; name: string } }>) {
    if (!link.shared_lib_id) continue;
    if (!repoMap.has(link.shared_lib_id)) repoMap.set(link.shared_lib_id, []);
    repoMap.get(link.shared_lib_id)!.push(link.repos);
  }

  const libsWithRepos = (libs ?? []).map((lib: any) => ({
    ...lib,
    repos: repoMap.get(lib.id) ?? [],
  }));

  return { libs: libsWithRepos };
};

export const actions: Actions = {
  add: async ({ request }) => {
    const db = getDb();
    const formData = await request.formData();
    const name = String(formData.get('name') ?? '').trim();
    const url = String(formData.get('url') ?? '').trim();

    if (!name) return fail(400, { error: 'Library name is required' });
    if (!url) return fail(400, { error: 'URL or path is required' });

    if (url.startsWith('http://') || url.startsWith('https://')) {
      try { new URL(url); } catch { return fail(400, { error: 'Invalid URL' }); }
    }

    const inferred = inferSourceType(url);
    const { source_type } = inferred;
    const base_url = 'base_url' in inferred ? inferred.base_url : null;
    const local_path = 'local_path' in inferred ? inferred.local_path : null;

    const { data: existing } = await db
      .from('shared_libs')
      .upsert(
        { name, source_type, base_url: base_url ?? null, local_path: local_path ?? null, index_status: 'pending' },
        { onConflict: 'name' }
      )
      .select('id,name,source_type,base_url,local_path')
      .single();

    if (!existing) return fail(500, { error: 'Failed to register library' });

    // Background index — returns immediately
    await startLibFetch(db, existing as { id: string; name: string; source_type: string; base_url: string | null; local_path: string | null });

    return { success: true };
  },
};
