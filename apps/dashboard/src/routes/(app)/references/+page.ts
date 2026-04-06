import type { PageLoad } from './$types';
import { supabase } from '$lib/supabase';

export const load: PageLoad = async () => {
  const { data: references } = await (supabase as any).schema('sensei')
    .from('references')
    .select('url, title, description, tags, fetched_at, modified_at')
    .order('modified_at', { ascending: false });

  return { references: references ?? [] };
};
