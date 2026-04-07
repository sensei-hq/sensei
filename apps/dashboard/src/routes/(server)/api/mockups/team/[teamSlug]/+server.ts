import { json, error } from '@sveltejs/kit';
import type { RequestHandler } from './$types';

function timeAgo(date: string): string {
  const diff = Date.now() - new Date(date).getTime();
  const m = Math.floor(diff / 60000);
  if (m < 60) return `${m}m ago`;
  const h = Math.floor(m / 60);
  if (h < 24) return `${h}h ago`;
  return `${Math.floor(h / 24)}d ago`;
}

export const GET: RequestHandler = async ({ fetch, params, locals }) => {
  const userId = locals.session?.user?.id;

  // Resolve team via the user_teams view (scoped to current user's memberships)
  const teamRes = await fetch(`/data/sensei/user_teams?slug=eq.${params.teamSlug}&user_id=eq.${userId}&limit=1`);
  const { data: userTeams } = await teamRes.json();
  const userTeam = userTeams?.[0];
  if (!userTeam) throw error(404, 'Team not found');

  // Fetch full team record to get team id
  const fullTeamRes = await fetch(
    `/data/core/teams?slug=eq.${params.teamSlug}&select=id,slug,display_name,account_id&limit=1`
  );
  const { data: teams } = await fullTeamRes.json();
  const team = teams?.[0];
  if (!team) throw error(404, 'Team not found');

  const [membersRes, pendingRes, reposRes] = await Promise.all([
    fetch(`/data/core/team_members?team_id=eq.${team.id}&select=role,user_id,profile:profiles(display_name,slug,avatar_url)`),
    fetch(`/data/core/invitations?account_id=eq.${team.account_id}&accepted_at=is.null`),
    fetch(`/data/sensei/team_repos?team_id=eq.${team.id}&order=last_indexed_at.desc`),
  ]);

  const { data: members } = await membersRes.json();
  const { data: pending } = await pendingRes.json();
  const { data: repos }   = await reposRes.json();

  type Member = { role: string; user_id: string; profile: { display_name: string; slug: string; avatar_url: string | null } };
  type Invite  = { email: string; created_at: string };

  return json({
    teamName: team.display_name,
    stats: [
      { label: 'Members',         value: String((members ?? []).length) },
      { label: 'Pending Invites', value: String((pending ?? []).length) },
    ],
    contributors: (members ?? [] as Member[]).map((m: Member) => ({
      initials: (m.profile?.display_name ?? '?').split(' ').map((w: string) => w[0]).join('').slice(0, 2).toUpperCase(),
      name:     m.profile?.display_name ?? m.user_id,
      slug:     m.profile?.slug,
      role:     m.role,
    })),
    repos: (repos ?? []).map((r: { repo_id: string; name: string; stack: string[] | null; last_indexed_at: string | null }) => ({
      name:    r.name,
      stack:   r.stack,
      indexed: r.last_indexed_at ? timeAgo(r.last_indexed_at) : 'Never',
    })),
    pending: (pending ?? [] as Invite[]).map((inv: Invite) => ({
      email:     inv.email,
      invited:   timeAgo(inv.created_at),
      status:    'Invite sent',
      statusCls: 'bg-warning-z1 text-warning-z7 border-warning-z3',
    })),
  });
};
