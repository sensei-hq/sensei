// apps/dashboard/src/routes/repos/[id]/analytics/+page.server.ts
import type { PageServerLoad } from './$types';
import { getDb } from '$lib/server/db';
import { error } from '@sveltejs/kit';

export const load: PageServerLoad = async ({ params }) => {
  const db = getDb();

  const { data: repo } = await db
    .from('repos')
    .select('id,name')
    .eq('id', params.id)
    .single();

  if (!repo) throw error(404, 'Repo not found');

  const since = new Date(Date.now() - 30 * 24 * 60 * 60 * 1000).toISOString();

  const { data: rawSessions } = await db
    .from('task_sessions')
    .select('id,session_id,task_description,task_type,status,ftr_score,ftr_signals,created_at,completed_at')
    .eq('repo_id', params.id)
    .gte('created_at', since)
    .order('created_at', { ascending: false })
    .limit(100);

  const { data: rawTurns } = await db
    .from('task_turns')
    .select('tool,success,duration_ms,task_session_id')
    .eq('repo_id', params.id)
    .gte('created_at', since);

  // Aggregate turns by tool client-side
  const toolMap = new Map<string, { calls: number; successes: number; totalDuration: number; durationCount: number }>();
  for (const turn of ((rawTurns ?? []) as Array<{ tool: string; success: boolean | null; duration_ms: number | null }>)) {
    const entry = toolMap.get(turn.tool) ?? { calls: 0, successes: 0, totalDuration: 0, durationCount: 0 };
    entry.calls++;
    if (turn.success === true) entry.successes++;
    if (turn.duration_ms !== null) { entry.totalDuration += turn.duration_ms; entry.durationCount++; }
    toolMap.set(turn.tool, entry);
  }

  const toolUsage = Array.from(toolMap.entries())
    .map(([tool, t]) => ({
      tool,
      calls: t.calls,
      successRate: t.calls > 0 ? Math.round((t.successes / t.calls) * 1000) / 1000 : 0,
      avgDurationMs: t.durationCount > 0 ? Math.round(t.totalDuration / t.durationCount) : null,
    }))
    .sort((a, b) => b.calls - a.calls)
    .slice(0, 10);

  const sessions = ((rawSessions ?? []) as Array<Record<string, unknown>>).map(s => ({
    id: s.id as string,
    sessionId: (s.session_id as string | null) ?? null,
    taskDescription: (s.task_description as string | null) ?? null,
    taskType: (s.task_type as string | null) ?? null,
    status: s.status as string,
    ftrScore: (s.ftr_score as number | null) ?? null,
    ftrSignals: (s.ftr_signals as Record<string, unknown> | null) ?? null,
    createdAt: s.created_at as string,
    completedAt: (s.completed_at as string | null) ?? null,
  }));

  return {
    repo: repo as { id: string; name: string },
    sessions,
    toolUsage,
  };
};
