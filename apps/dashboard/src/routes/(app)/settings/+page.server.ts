import type { PageServerLoad } from './$types';

export const load: PageServerLoad = async ({ fetch, locals }) => {
  const user   = locals.session?.user;
  const userId = user?.id;

  if (!userId) return { user: null, orgs: [], apiToken: null, daemonStatus: null };

  const [profileRes, orgsRes] = await Promise.all([
    fetch(`/data/core/profiles?user_id=eq.${userId}&limit=1`),
    fetch(`/data/core/profile_accounts?user_id=eq.${userId}&select=role,account:accounts(id,slug,display_name,account_type)`),
  ]);

  const { data: profiles }    = await profileRes.json();
  const { data: memberships } = await orgsRes.json();

  const profile = profiles?.[0];

  return {
    user: {
      id:        userId,
      name:      profile?.display_name ?? user?.full_name ?? user?.email,
      email:     user?.email ?? '',
      avatarUrl: profile?.avatar_url ?? null,
    },
    orgs: (memberships ?? []).map((m: { role: string; account: { id: string; slug: string; display_name: string; account_type: string } }) => ({
      id:   m.account?.id,
      name: m.account?.display_name,
      slug: m.account?.slug,
      plan: m.account?.account_type,
      role: m.role,
    })),
    apiToken:    null,
    daemonStatus: null,
  };
};
