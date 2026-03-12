import type { PageLoad } from './$types';
import { supabase } from '$lib/supabase';

export const load: PageLoad = async () => {
  const { data: repos } = await (supabase as any).schema('sensei')
    .from('repos')
    .select('id, name, remote_url, default_branch, stack, description, last_indexed_commit, last_indexed_at, is_public')
    .order('last_indexed_at', { ascending: false });

  return { repos: repos ?? [] };
};
