import type { PageServerLoad } from './$types';
import { createClient } from '@supabase/supabase-js';
import { SUPABASE_URL, SUPABASE_SERVICE_ROLE_KEY } from '$env/static/private';

function getCoreDb() {
  return createClient(SUPABASE_URL, SUPABASE_SERVICE_ROLE_KEY, {
    db: { schema: 'core' },
    auth: { persistSession: false },
  });
}

function getSenseiDb() {
  return createClient(SUPABASE_URL, SUPABASE_SERVICE_ROLE_KEY, {
    db: { schema: 'sensei' },
    auth: { persistSession: false },
  });
}

export interface MemberFtr {
  userId: string;
  email: string;
  role: string;
  avgFtr: number | null;
  sessionCount: number;
  repoBreakdown: Array<{ repoId: string; repoName: string; avgFtr: number | null; sessionCount: number }>;
}

export const load: PageServerLoad = async () => {
  const since = new Date(Date.now() - 30 * 24 * 60 * 60 * 1000).toISOString();

  let members: MemberFtr[] = [];

  try {
    const coreDb = getCoreDb();
    const senseiDb = getSenseiDb();

    // Load profile_accounts to get all members of this account
    const { data: profileAccounts } = await coreDb
      .from('profile_accounts')
      .select('user_id, role, profiles(email)');

    if (profileAccounts && profileAccounts.length > 0) {
      // Load task sessions for all users in the last 30 days
      const { data: sessions } = await senseiDb
        .from('task_sessions')
        .select('id, user_id, repo_id, ftr_score, created_at')
        .gte('created_at', since)
        .not('ftr_score', 'is', null);

      // Load repos for name lookup
      const { data: repos } = await senseiDb
        .from('repos')
        .select('id, name');

      const repoMap = new Map((repos ?? []).map((r: { id: string; name: string }) => [r.id, r.name]));

      members = (profileAccounts as unknown as Array<{
        user_id: string;
        role: string;
        profiles: { email: string } | Array<{ email: string }> | null;
      }>).map(pa => {
        const userSessions = (sessions ?? []).filter((s: { user_id: string | null }) => s.user_id === pa.user_id);

        const scored = userSessions.filter((s: { ftr_score: number | null }) => s.ftr_score !== null);
        const avgFtr = scored.length > 0
          ? scored.reduce((sum: number, s: { ftr_score: number | null }) => sum + (s.ftr_score ?? 0), 0) / scored.length
          : null;

        // Per-repo breakdown
        const repoGroups = new Map<string, Array<{ ftr_score: number | null }>>();
        for (const s of userSessions as Array<{ repo_id: string; ftr_score: number | null }>) {
          const existing = repoGroups.get(s.repo_id) ?? [];
          existing.push(s);
          repoGroups.set(s.repo_id, existing);
        }

        const repoBreakdown = Array.from(repoGroups.entries()).map(([repoId, repoSessions]) => {
          const repoScored = repoSessions.filter(s => s.ftr_score !== null);
          const repoAvgFtr = repoScored.length > 0
            ? repoScored.reduce((sum, s) => sum + (s.ftr_score ?? 0), 0) / repoScored.length
            : null;
          return {
            repoId,
            repoName: repoMap.get(repoId) ?? repoId,
            avgFtr: repoAvgFtr,
            sessionCount: repoSessions.length,
          };
        });

        const profileEmail = Array.isArray(pa.profiles) ? pa.profiles[0]?.email : pa.profiles?.email;
        const email = profileEmail ?? pa.user_id;
        // Mask email: show first 3 chars then ***@domain
        const maskedEmail = email.includes('@')
          ? email.slice(0, 3) + '***@' + email.split('@')[1]
          : email;

        return {
          userId: pa.user_id,
          email: maskedEmail,
          role: pa.role,
          avgFtr,
          sessionCount: userSessions.length,
          repoBreakdown,
        };
      });

      // Sort by avgFtr descending (nulls last), then by sessionCount
      members.sort((a, b) => {
        if (a.avgFtr !== null && b.avgFtr !== null) return b.avgFtr - a.avgFtr;
        if (a.avgFtr !== null) return -1;
        if (b.avgFtr !== null) return 1;
        return b.sessionCount - a.sessionCount;
      });
    }
  } catch {
    // Tables may not exist yet — return empty data gracefully
  }

  return { members };
};
