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

export interface Member {
  userId: string;
  email: string;
  role: string;
  lastActive: string | null;
  avgFtr: number | null;
  sessionCount: number;
}

export const load: PageServerLoad = async () => {
  let members: Member[] = [];

  try {
    const coreDb = getCoreDb();
    const senseiDb = getSenseiDb();

    const { data: profileAccounts } = await coreDb
      .from('profile_accounts')
      .select('user_id, role, profiles(email)')
      .order('role', { ascending: true });

    if (profileAccounts && profileAccounts.length > 0) {
      const since = new Date(Date.now() - 30 * 24 * 60 * 60 * 1000).toISOString();

      const { data: sessions } = await senseiDb
        .from('task_sessions')
        .select('user_id, ftr_score, created_at')
        .gte('created_at', since);

      members = (profileAccounts as unknown as Array<{
        user_id: string;
        role: string;
        profiles: { email: string } | Array<{ email: string }> | null;
      }>).map(pa => {
        const userSessions = (sessions ?? []).filter(
          (s: { user_id: string | null }) => s.user_id === pa.user_id
        );
        const scored = userSessions.filter((s: { ftr_score: number | null }) => s.ftr_score !== null);
        const avgFtr = scored.length > 0
          ? scored.reduce((sum: number, s: { ftr_score: number | null }) => sum + (s.ftr_score ?? 0), 0) / scored.length
          : null;

        const lastActive = userSessions.length > 0
          ? userSessions.reduce(
              (latest: string, s: { created_at: string }) =>
                s.created_at > latest ? s.created_at : latest,
              userSessions[0].created_at
            )
          : null;

        return {
          userId: pa.user_id,
          email: (Array.isArray(pa.profiles) ? pa.profiles[0]?.email : pa.profiles?.email) ?? pa.user_id,
          role: pa.role,
          lastActive,
          avgFtr,
          sessionCount: userSessions.length,
        };
      });
    }
  } catch {
    // Tables may not exist yet — return empty data gracefully
  }

  return { members };
};
