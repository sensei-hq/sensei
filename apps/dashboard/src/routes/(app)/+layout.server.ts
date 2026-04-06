import type { LayoutServerLoad } from './$types';

export const load: LayoutServerLoad = async ({ parent, fetch }) => {
  const { session } = await parent();
  if (!session?.user?.id) return { teams: [] };

  const res = await fetch(`/data?entity=user_teams&user_id=eq.${session.user.id}`);
  if (!res.ok) return { teams: [] };

  const body = await res.json();
  const rows: { slug: string; display_name: string; account_slug: string; role: string }[] = body.data ?? [];

  return {
    teams: rows.map((r) => ({
      slug: r.slug,
      displayName: r.display_name,
      accountSlug: r.account_slug,
      role: r.role,
    })),
  };
};
