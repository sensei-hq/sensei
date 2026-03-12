import type { PageLoad } from './$types';
import { supabase } from '$lib/supabase';

export const load: PageLoad = async () => {
  const db = (supabase as any).schema('sensei');
  const [{ count: repoCount }, { data: recentEvents }, { data: repos }] = await Promise.all([
    db.from('repos').select('*', { count: 'exact', head: true }),
    db.from('events').select('tool, ts, phase')
      .order('ts', { ascending: false }).limit(10),
    db.from('repos').select('name, last_indexed_at, stack')
      .order('last_indexed_at', { ascending: false }).limit(5),
  ]);

  return { repoCount: repoCount ?? 0, recentEvents: recentEvents ?? [], repos: repos ?? [] };
};
