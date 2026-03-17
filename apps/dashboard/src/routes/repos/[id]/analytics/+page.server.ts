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

  const { data: rawSessions, error: sessionsError } = await db
    .from('task_sessions')
    .select('id,session_id,task_description,task_type,status,ftr_score,ftr_signals,created_at,completed_at')
    .eq('repo_id', params.id)
    .gte('created_at', since)
    .order('created_at', { ascending: false })
    .limit(100);
  if (sessionsError) throw error(500, sessionsError.message);

  // created_at is used as a filter only (PostgREST allows filtering on columns not in the select projection)
  const { data: rawTurns, error: turnsError } = await db
    .from('task_turns')
    .select('tool,success,duration_ms')
    .eq('repo_id', params.id)
    .gte('created_at', since);
  if (turnsError) throw error(500, turnsError.message);

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

  // Cost data from OTel api_requests — last 30 days
  const { data: apiRequestRows } = await db
    .from("api_requests")
    .select("task_session_id, input_tokens, output_tokens, cache_read_tokens, cache_creation_tokens, cost_usd, recorded_at")
    .eq("repo_id", params.id)
    .gte("recorded_at", since)
    .order("recorded_at", { ascending: false });

  // Aggregate cost per task_session_id client-side
  const costBySession = new Map<string, { inputTokens: number; outputTokens: number; cacheReadTokens: number; cacheCreationTokens: number; costUsd: number }>();
  for (const row of ((apiRequestRows ?? []) as Array<{ task_session_id: string | null; input_tokens: number; output_tokens: number; cache_read_tokens: number; cache_creation_tokens: number; cost_usd: string | number }>) ) {
    if (!row.task_session_id) continue;
    const existing = costBySession.get(row.task_session_id) ?? { inputTokens: 0, outputTokens: 0, cacheReadTokens: 0, cacheCreationTokens: 0, costUsd: 0 };
    costBySession.set(row.task_session_id, {
      inputTokens: existing.inputTokens + (row.input_tokens ?? 0),
      outputTokens: existing.outputTokens + (row.output_tokens ?? 0),
      cacheReadTokens: existing.cacheReadTokens + (row.cache_read_tokens ?? 0),
      cacheCreationTokens: existing.cacheCreationTokens + (row.cache_creation_tokens ?? 0),
      costUsd: existing.costUsd + Number(row.cost_usd ?? 0),
    });
  }

  // Benchmark runs — all time, paired by (task_description, branch)
  const { data: benchmarkRunRows } = await db
    .from("benchmark_runs")
    .select("id, task_description, branch, sensei_enabled, started_at, ended_at, total_cost_usd, total_input_tokens, total_output_tokens")
    .eq("repo_id", params.id)
    .order("started_at", { ascending: false });

  type BenchmarkEntry = { costUsd: number; inputTokens: number; outputTokens: number };
  type BenchmarkPair = {
    taskDescription: string;
    branch: string;
    withSensei: BenchmarkEntry | null;
    withoutSensei: BenchmarkEntry | null;
  };
  const pairMap = new Map<string, BenchmarkPair>();
  for (const run of ((benchmarkRunRows ?? []) as Array<{ task_description: string; branch: string; sensei_enabled: boolean; total_cost_usd: string | number | null; total_input_tokens: number | null; total_output_tokens: number | null }>) ) {
    const key = `${run.task_description}::${run.branch}`;
    const pair = pairMap.get(key) ?? { taskDescription: run.task_description, branch: run.branch, withSensei: null, withoutSensei: null };
    const entry: BenchmarkEntry = {
      costUsd: Number(run.total_cost_usd ?? 0),
      inputTokens: run.total_input_tokens ?? 0,
      outputTokens: run.total_output_tokens ?? 0,
    };
    if (run.sensei_enabled) pair.withSensei = entry;
    else pair.withoutSensei = entry;
    pairMap.set(key, pair);
  }
  const benchmarkPairs = Array.from(pairMap.values()).filter(p => p.withSensei !== null && p.withoutSensei !== null);

  return {
    repo: repo as { id: string; name: string },
    sessions,
    toolUsage,
    costBySession: Object.fromEntries(costBySession),
    benchmarkPairs,
  };
};
