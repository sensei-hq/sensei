import type { SupabaseClient } from "@supabase/supabase-js";

export interface ToolStat {
  name: string;
  calls: number;
  success_rate: number;
  avg_duration_ms: number;
  last_called?: number;
}

export interface StatsResult {
  period: { from: string; to: string };
  total_calls: number;
  tools: ToolStat[];
  sessions: number;
  projects: number;
  // Populated only for specific flag combinations:
  tool?: ToolStat;
  events?: Array<Record<string, unknown>>;
}

export interface StatsOptions {
  all?: boolean;
  tool?: string;
  session?: string;
  since?: string;   // YYYY-MM-DD
}

export async function queryStats(client: SupabaseClient, opts: StatsOptions): Promise<StatsResult> {
  const now = Date.now();
  const toDate = new Date(now).toISOString().slice(0, 10);

  // Session mode: return chronological events for that session (no date filter)
  if (opts.session) {
    const { data: events } = await client
      .from("events")
      .select("*")
      .eq("session_id", opts.session)
      .order("ts", { ascending: true });

    const rows = (events ?? []).map((e: any) => ({ ...e, ts: new Date(e.ts as string).getTime() }));
    const firstTs = (rows[0]?.ts as number | undefined) ?? now;

    return {
      period: { from: new Date(firstTs).toISOString().slice(0, 10), to: toDate },
      total_calls: rows.length,
      tools: [],
      sessions: 1,
      projects: 0,
      events: rows,
    };
  }

  // Determine date filter
  const since: string | undefined = opts.all
    ? undefined
    : opts.since
      ? new Date(opts.since).toISOString()
      : new Date(Date.now() - 7 * 24 * 60 * 60 * 1000).toISOString();

  // Build query for post-phase events
  let query = client.from("events").select("*").eq("phase", "post");
  if (since) query = query.gte("ts", since);
  if (opts.tool) query = query.eq("tool", opts.tool);

  const { data: events } = await query;
  const rows = events ?? [];

  // Aggregate client-side
  const toolCounts = new Map<string, { calls: number; successes: number; totalDuration: number; lastTs: number }>();
  for (const e of rows) {
    const entry = toolCounts.get(e.tool) ?? { calls: 0, successes: 0, totalDuration: 0, lastTs: 0 };
    entry.calls++;
    if (e.success === true) entry.successes++;
    if (e.duration_ms != null) entry.totalDuration += e.duration_ms;
    entry.lastTs = Math.max(entry.lastTs, new Date(e.ts as string).getTime());
    toolCounts.set(e.tool, entry);
  }

  const tools: ToolStat[] = Array.from(toolCounts.entries())
    .map(([name, s]) => ({
      name,
      calls: s.calls,
      success_rate: Math.round((s.calls > 0 ? s.successes / s.calls : 0) * 100) / 100,
      avg_duration_ms: Math.round(s.calls > 0 ? s.totalDuration / s.calls : 0),
      last_called: s.lastTs,
    }))
    .sort((a, b) => b.calls - a.calls);

  const total_calls = rows.length;
  const sessions = new Set(rows.map((e: any) => e.session_id)).size;
  const projects = new Set(rows.map((e: any) => e.project_path)).size;

  const fromTs = opts.all
    ? (rows.length > 0 ? Math.min(...rows.map((e: any) => new Date(e.ts as string).getTime())) : now)
    : since
      ? new Date(since).getTime()
      : now - 7 * 24 * 60 * 60 * 1000;

  const result: StatsResult = {
    period: { from: new Date(fromTs).toISOString().slice(0, 10), to: toDate },
    total_calls,
    tools,
    sessions,
    projects,
  };

  if (opts.tool && tools.length > 0) {
    result.tool = tools[0];
  }

  return result;
}
