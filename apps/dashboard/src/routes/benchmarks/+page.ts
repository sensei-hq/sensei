import type { PageLoad } from './$types';
import { supabase } from '$lib/supabase';

export const load: PageLoad = async () => {
  const { data: reports } = await (supabase as any).schema('sensei')
    .from('benchmark_reports')
    .select('id, run_name, strategy, score, tokens, elapsed_ms, promoted, created_at, repo_id')
    .order('created_at', { ascending: false })
    .limit(50);

  return { reports: reports ?? [] };
};
