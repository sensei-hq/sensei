import type { PageServerLoad } from './$types';

async function getData<T>(
  fetch: typeof globalThis.fetch,
  entity: string,
  params: Record<string, string> = {}
): Promise<T[]> {
  const query = new URLSearchParams({ entity, ...params });
  const res = await fetch(`/data?${query}`);
  if (!res.ok) return [];
  const body = await res.json() as { data?: T[] };
  return body.data ?? [];
}

export const load: PageServerLoad = async ({ fetch, parent }) => {
  const { session } = await parent();

  const since = new Date(Date.now() - 30 * 24 * 60 * 60 * 1000).toISOString();

  const [rawRepos, rawSessions, rawLibs] = await Promise.all([
    getData<Record<string, unknown>>(fetch, 'repos', {
      ':select': 'id,name,stack,last_indexed_at',
      ':order': 'created_at:desc',
    }),
    getData<Record<string, unknown>>(fetch, 'task_sessions', {
      ':select': 'id,repo_id,task_description,status,ftr_score,created_at,completed_at',
      ':order': 'created_at:desc',
      ':limit': '50',
      'created_at': `gte.${since}`,
    }),
    getData<Record<string, unknown>>(fetch, 'libraries', {
      ':select': 'id,name,source_type,section_count,index_status',
      ':order': 'name:asc',
    }),
  ]);

  // Session count per repo from already-fetched sessions
  const sessionCountByRepo: Record<string, number> = {};
  for (const s of rawSessions) {
    const rid = s.repo_id as string;
    sessionCountByRepo[rid] = (sessionCountByRepo[rid] ?? 0) + 1;
  }

  const user = session?.user ?? { id: 'unknown', name: 'User', email: '', role: 'user' };
  const userName = user.name ?? user.email ?? 'User';

  return {
    user: {
      name: userName,
      email: user.email ?? '',
      initials: userName.charAt(0).toUpperCase(),
    },
    workspaces: [
      { id: 'personal', name: userName, type: 'personal' as const },
    ],
    repos: rawRepos.map(r => ({
      id: r.id as string,
      name: r.name as string,
      stack: (r.stack as string[] | null) ?? [],
      lastIndexedAt: (r.last_indexed_at as string | null) ?? null,
      sessionCount: sessionCountByRepo[r.id as string] ?? 0,
    })),
    taskSessions: rawSessions.map(s => ({
      id: s.id as string,
      repoId: s.repo_id as string,
      taskDescription: (s.task_description as string | null) ?? '(no description)',
      status: s.status as string,
      ftrScore: (s.ftr_score as number | null) ?? null,
      createdAt: s.created_at as string,
      completedAt: (s.completed_at as string | null) ?? null,
    })),
    libraries: rawLibs.map(l => ({
      id: l.id as string,
      name: l.name as string,
      sourceType: (l.source_type as string | null) ?? 'unknown',
      sectionCount: (l.section_count as number | null) ?? 0,
      indexStatus: (l.index_status as string | null) ?? 'unknown',
    })),
  };
};
