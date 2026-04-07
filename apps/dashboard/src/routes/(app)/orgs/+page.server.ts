import type { PageServerLoad } from './$types';

export const load: PageServerLoad = async ({ fetch, locals }) => {
  const userId = locals.session?.user?.id;
  if (!userId) return { orgs: [] };

  const res = await fetch(
    `/data/core/profile_accounts?user_id=eq.${userId}&select=role,account:accounts(id,slug,display_name,account_type,created_at)`
  );
  const { data: memberships } = await res.json();

  return {
    orgs: (memberships ?? []).map((m: { role: string; account: { id: string; slug: string; display_name: string; account_type: string; created_at: string } }) => ({
      id:   m.account?.id,
      name: m.account?.display_name,
      slug: m.account?.slug,
      plan: m.account?.account_type,
      role: m.role,
    })),
  };
};
