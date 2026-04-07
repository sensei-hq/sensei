import type { PageServerLoad } from './$types';
import { error } from '@sveltejs/kit';

export const load: PageServerLoad = async ({ fetch, params }) => {
  // params.id is the account slug
  const orgRes = await fetch(`/data/core/accounts?slug=eq.${params.id}&limit=1`);
  const { data: orgs } = await orgRes.json();
  const org = orgs?.[0];
  if (!org) throw error(404, 'Organisation not found');

  const [teamsRes, membersRes, reposRes] = await Promise.all([
    fetch(`/data/core/teams?account_id=eq.${org.id}&order=slug.asc`),
    fetch(`/data/core/profile_accounts?account_id=eq.${org.id}&select=role,joined_at,profile:profiles(user_id,display_name,slug,avatar_url)`),
    fetch(`/data/sensei/account_repos?account_id=eq.${org.id}&order=last_indexed_at.desc`),
  ]);

  const { data: teams }   = await teamsRes.json();
  const { data: members } = await membersRes.json();
  const { data: repos }   = await reposRes.json();

  return {
    org: {
      id:        org.id,
      name:      org.display_name,
      slug:      org.slug,
      plan:      org.account_type,
      createdAt: org.created_at,
    },
    teams: (teams ?? []).map((t: { id: string; slug: string; display_name: string }) => ({
      id:   t.id,
      name: t.display_name,
      slug: t.slug,
    })),
    members: (members ?? []).map((m: { role: string; joined_at: string; profile: { user_id: string; display_name: string; slug: string; avatar_url: string | null } }) => ({
      id:        m.profile?.user_id,
      name:      m.profile?.display_name,
      slug:      m.profile?.slug,
      avatarUrl: m.profile?.avatar_url ?? null,
      role:      m.role,
      joinedAt:  m.joined_at,
    })),
    repos: (repos ?? []).map((r: { repo_id: string; name: string; remote_url: string | null; stack: string[] | null; last_indexed_at: string | null }) => ({
      id:            r.repo_id,
      name:          r.name,
      remoteUrl:     r.remote_url,
      stack:         r.stack,
      lastIndexedAt: r.last_indexed_at,
    })),
  };
};
