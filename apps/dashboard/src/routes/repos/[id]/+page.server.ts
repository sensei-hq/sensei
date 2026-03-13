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

  return { repo, symbols: symbols ?? [] };
};
