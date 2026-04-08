import type { PageServerLoad } from './$types';

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

export const load: PageServerLoad = async ({ fetch, locals }) => {
  const user = locals.session?.user;
  const userId = user?.id;

  if (!userId) {
    return { userName: 'You', orgName: '', email: '', ftr: null, stats: [], repos: [], sessions: [], setupSteps: [], setupDoneUpTo: 0 };
  }

  const since30d = new Date(Date.now() - 30 * 24 * 60 * 60 * 1000).toISOString();

  const [reposRes, sessionsRes, accountRes, costRes] = await Promise.all([
    fetch(`/data/sensei/repos?owner_id=eq.${userId}&order=last_indexed_at.desc&limit=5`),
    fetch(`/data/sensei/task_sessions?select=id,task_description,status,ftr_score,created_at,repo:repos(name)&order=created_at.desc&limit=10`),
    fetch(`/data/core/profile_accounts?user_id=eq.${userId}&select=role,account:accounts(slug,display_name)&limit=1`),
    fetch(`/data/sensei/api_requests?select=cost_usd,cache_read_tokens,input_tokens&recorded_at=gte.${since30d}`),
  ]);

  const { data: repos }       = await reposRes.json();
  const { data: sessions }    = await sessionsRes.json();
  const { data: accounts }    = await accountRes.json();
  const { data: apiRequests } = await costRes.json();

  const isNewUser = (repos ?? []).length === 0 && (sessions ?? []).length === 0;
  const orgName = accounts?.[0]?.account?.display_name ?? '';
  const orgRole = accounts?.[0]?.role ?? '';

  type Session = { ftr_score: number | null; status: string; created_at: string; task_description: string | null; repo: { name: string } | null };
  const allSessions = (sessions ?? []) as Session[];
  const recent    = allSessions.filter(s => new Date(s.created_at).getTime() > Date.parse(since30d));
  const completed = recent.filter(s => s.ftr_score != null);
  const avgFtr    = completed.length
    ? Math.round(completed.reduce((sum, s) => sum + Number(s.ftr_score), 0) / completed.length * 100)
    : null;

  type ApiReq = { cost_usd: number; cache_read_tokens: number; input_tokens: number };
  const reqs        = (apiRequests ?? []) as ApiReq[];
  const totalCost   = reqs.reduce((sum, r) => sum + Number(r.cost_usd), 0);
  const totalInput  = reqs.reduce((sum, r) => sum + r.input_tokens, 0);
  const totalCache  = reqs.reduce((sum, r) => sum + r.cache_read_tokens, 0);
  const cacheHit    = totalInput + totalCache > 0
    ? Math.round(totalCache / (totalInput + totalCache) * 100)
    : null;

  const stats = [
    { label: 'Sessions (30d)', value: String(recent.length)            },
    ...(avgFtr    != null ? [{ label: 'Avg FTR (30d)',  value: `${avgFtr}%`               }] : []),
    ...(totalCost  > 0    ? [{ label: 'Cost (30d)',      value: `$${totalCost.toFixed(2)}` }] : []),
    ...(cacheHit  != null ? [{ label: 'Cache Hit Rate',  value: `${cacheHit}%`             }] : []),
  ];

  return {
    userName: user?.full_name ?? user?.email ?? 'You',
    orgName,
    orgRole,
    isNewUser,
    email:    user?.email ?? '',
    ftr:      avgFtr,
    stats,
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
  };
};
