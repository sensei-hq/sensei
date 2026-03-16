import type { PageServerLoad } from './$types';
import { error } from '@sveltejs/kit';
import { getDb } from '$lib/server/db';

export const load: PageServerLoad = async ({ params }) => {
  const db = getDb();

  const { data: repo } = await db
    .from('repos')
    .select('*')
    .eq('id', params.id)
    .single();

  if (!repo) throw error(404, 'Repo not found');

  const { data: symbols } = await db
    .from('symbols')
    .select('name,kind,file_path,line_start,is_exported,signature')
    .eq('repo_id', params.id)
    .eq('is_exported', true)
    .order('file_path')
    .order('line_start')
    .limit(500);

  // Count libs needing attention (missing or stale)
  const { data: repoLibs } = await db
    .from('repo_libs')
    .select('name')
    .eq('repo_id', params.id);

  const { data: libSections, error: libSectionsErr } = await db
    .from('lib_doc_sections')
    .select('lib_name,last_fetched')
    .eq('repo_id', params.id)
    .limit(10000);

  // If the sections query fails, fall back to 0 to avoid showing a false attention badge
  const libAttentionCount = libSectionsErr ? 0 : (() => {
    const libSectionMap = new Map<string, string>();
    for (const s of (libSections ?? []) as Array<{ lib_name: string; last_fetched: string }>) {
      const existing = libSectionMap.get(s.lib_name);
      if (!existing || s.last_fetched > existing) libSectionMap.set(s.lib_name, s.last_fetched);
    }
    const STALE_MS = 7 * 24 * 60 * 60 * 1000;
    return (repoLibs ?? []).filter((l: { name: string }) => {
      const lastFetched = libSectionMap.get(l.name);
      if (!lastFetched) return true;
      return Date.now() - new Date(lastFetched).getTime() > STALE_MS;
    }).length;
  })();

  return { repo, symbols: symbols ?? [], libAttentionCount };
};
