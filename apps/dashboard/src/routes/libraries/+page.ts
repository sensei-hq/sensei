import type { PageLoad } from './$types';
import { supabase } from '$lib/supabase';

export const load: PageLoad = async () => {
  const { data: libraries } = await (supabase as any).schema('sensei')
    .from('libraries')
    .select('name, ecosystem, version, description, homepage_url, llms_txt_url, llms_txt_fetched_at')
    .order('ecosystem')
    .order('name');

  return { libraries: libraries ?? [] };
};
