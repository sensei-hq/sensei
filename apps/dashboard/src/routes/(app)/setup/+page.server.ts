import type { PageServerLoad } from './$types';

export const load: PageServerLoad = async ({ fetch, locals }) => {
  const user = locals.session?.user;
  const userId = user?.id;
  if (!userId) return { userName: 'You', orgName: '', hasRepos: false, hasSessions: false };

  const [reposRes, sessionsRes, accountRes] = await Promise.all([
    fetch(`/data/sensei/repos?owner_id=eq.${userId}&select=id&limit=1`),
    fetch(`/data/sensei/task_sessions?select=id&limit=1`),
    fetch(`/data/core/profile_accounts?user_id=eq.${userId}&select=role,account:accounts(slug,display_name)&limit=1`),
  ]);

  const { data: repos }    = await reposRes.json();
  const { data: sessions } = await sessionsRes.json();
  const { data: accounts } = await accountRes.json();

  const hasRepos    = (repos ?? []).length > 0;
  const hasSessions = (sessions ?? []).length > 0;
  const orgName     = accounts?.[0]?.account?.display_name ?? '';

  return {
    userName:    user?.full_name ?? user?.email ?? 'You',
    orgName,
    hasRepos,
    hasSessions,
  };
};
