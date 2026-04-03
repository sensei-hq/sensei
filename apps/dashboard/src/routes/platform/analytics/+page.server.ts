// apps/dashboard/src/routes/platform/analytics/+page.server.ts
import type { PageServerLoad } from './$types';
import { createClient } from '@supabase/supabase-js';
import { SUPABASE_URL, SUPABASE_SERVICE_ROLE_KEY } from '$env/static/private';

function getPlatformDb() {
  return createClient(SUPABASE_URL, SUPABASE_SERVICE_ROLE_KEY, {
    db: { schema: 'platform' },
    auth: { persistSession: false },
  });
}

export interface FtrBucket {
  bucket: number | null;
  bucket_min: number | null;
  bucket_max: number | null;
  session_count: number | null;
}

export interface AccountStat {
  account_id: string | null;
  account_slug: string | null;
  account_type: string | null;
  repo_count: number | null;
  sessions_30d: number | null;
  avg_ftr: number | null;
  total_cost_usd: number | null;
}

export interface ToolStat {
  tool: string | null;
  total_calls: number | null;
  error_rate: number | null;
  avg_duration_ms: number | null;
}

export const load: PageServerLoad = async () => {
  const db = getPlatformDb();

  let ftrDistribution: FtrBucket[] = [];
  let accountStats: AccountStat[] = [];
  let toolUsage: ToolStat[] = [];

  try {
    const { data } = await db.from('ftr_distribution' as never).select('*') as { data: FtrBucket[] | null };
    if (data) ftrDistribution = data;
  } catch {
    // view may not exist yet
  }

  try {
    const { data } = await db.from('account_stats' as never).select('*') as { data: AccountStat[] | null };
    if (data) accountStats = data;
  } catch {
    // view may not exist yet
  }

  try {
    const { data } = await db.from('tool_usage' as never).select('*') as { data: ToolStat[] | null };
    if (data) toolUsage = data;
  } catch {
    // view may not exist yet
  }

  return { ftrDistribution, accountStats, toolUsage };
};
