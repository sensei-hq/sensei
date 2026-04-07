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

function ftrLabel(score: number | null, status: string): string {
  if (status === 'in_progress') return 'In progress';
  if (score === null) return 'No score';
  if (score >= 0.9) return 'First try';
  return `${Math.round(score * 100)}% FTR`;
}

export const GET: RequestHandler = async ({ fetch, params }) => {
  const profileRes = await fetch(`/data/core/profiles?slug=eq.${params.userSlug}&limit=1`);
  const { data: profiles } = await profileRes.json();
  const profile = profiles?.[0];
  if (!profile) throw error(404, 'User not found');

  const since30d = new Date(Date.now() - 30 * 24 * 60 * 60 * 1000).toISOString();

  const [reposRes, sessionsRes, accountRes] = await Promise.all([
    fetch(`/data/sensei/repos?owner_id=eq.${profile.user_id}&order=last_indexed_at.desc&limit=5`),
    fetch(`/data/sensei/task_sessions?select=id,task_description,status,ftr_score,created_at,repo:repos(name)&order=created_at.desc&limit=10`),
    fetch(`/data/core/profile_accounts?user_id=eq.${profile.user_id}&select=account:accounts(display_name)&limit=1`),
  ]);

  const { data: repos }    = await reposRes.json();
  const { data: sessions } = await sessionsRes.json();
  const { data: accounts } = await accountRes.json();

  const orgName = accounts?.[0]?.account?.display_name ?? '';

  type Session = { ftr_score: number | null; status: string; created_at: string; task_description: string | null; repo: { name: string } | null };
  const allSessions = (sessions ?? []) as Session[];
  const recent      = allSessions.filter(s => new Date(s.created_at).getTime() > Date.parse(since30d));
  const completed   = recent.filter(s => s.ftr_score != null);
  const avgFtr      = completed.length
    ? Math.round(completed.reduce((sum, s) => sum + Number(s.ftr_score), 0) / completed.length * 100)
    : null;

  return json({
    userName: profile.display_name,
    orgName,
    email:    null, // not exposed for other users
    ftr:      avgFtr,
    stats: [
      { label: 'Sessions (30d)', value: String(recent.length)                                  },
      ...(avgFtr != null ? [{ label: 'Avg FTR (30d)', value: `${avgFtr}%` }] : []),
    ],
    repos: (repos ?? []).map((r: { name: string; stack: string[] | null; last_indexed_at: string | null }) => ({
      name:    r.name,
      lang:    r.stack?.[0] ?? 'Unknown',
      indexed: r.last_indexed_at ? timeAgo(r.last_indexed_at) : 'Never',
    })),
    sessions: allSessions.map(s => ({
      task:   s.task_description ?? 'Untitled task',
      repo:   s.repo?.name ?? '',
      status: s.status,
      result: ftrLabel(s.ftr_score, s.status),
      when:   timeAgo(s.created_at),
    })),
    setupSteps:    ['Install CLI', 'Init repo', 'MCP setup', 'Link account', 'Plugin'],
    setupDoneUpTo: 3,
  });
};
