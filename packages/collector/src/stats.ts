import { Database } from "bun:sqlite";

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

/**
 * Build composable WHERE conditions.
 * --since can combine with --tool or --session per the spec.
 * --session alone has no date filter (returns all events for that session).
 */
function buildConditions(opts: StatsOptions): { conditions: string[]; params: unknown[] } {
  const conditions: string[] = [];
  const params: unknown[] = [];

  if (opts.session) {
    conditions.push("session_id = ?");
    params.push(opts.session);
  }

  if (opts.since) {
    conditions.push("ts >= ?");
    params.push(new Date(opts.since).getTime());
  } else if (!opts.all && !opts.session) {
    // Default: last 7 days — NOT applied when --session is active,
    // because --session must return all events for that session regardless of age.
    conditions.push("ts >= ?");
    params.push(Date.now() - 7 * 24 * 60 * 60 * 1000);
  }

  if (opts.tool) {
    conditions.push("tool = ?");
    params.push(opts.tool);
  }

  return { conditions, params };
}

export function queryStats(db: Database, opts: StatsOptions): StatsResult {
  const now = Date.now();
  const toDate = new Date(now).toISOString().slice(0, 10);

  const { conditions, params } = buildConditions(opts);
  const baseWhere = conditions.length > 0 ? conditions.join(" AND ") : "1";

  // For session mode, return the chronological event list
  if (opts.session) {
    const events = db
      .prepare(`SELECT * FROM events WHERE ${baseWhere} ORDER BY ts ASC`)
      .all(...params) as Array<Record<string, unknown>>;
    const firstTs = (events[0]?.ts as number | undefined) ?? now;
    return {
      period: { from: new Date(firstTs).toISOString().slice(0, 10), to: toDate },
      total_calls: events.length,
      tools: [],
      sessions: 1,
      projects: 0,
      events,
    };
  }

  const postWhere = `${baseWhere} AND phase = 'post'`;

  const totalRow = db
    .prepare(`SELECT COUNT(*) as n FROM events WHERE ${postWhere}`)
    .get(...params) as { n: number };
  const total_calls = totalRow.n;

  const toolRows = db.prepare(`
    SELECT
      tool,
      COUNT(*) as calls,
      AVG(CASE WHEN success = 1 THEN 1.0 ELSE 0.0 END) as success_rate,
      AVG(CASE WHEN duration_ms IS NOT NULL THEN duration_ms ELSE NULL END) as avg_duration_ms,
      MAX(ts) as last_called
    FROM events
    WHERE ${postWhere}
    GROUP BY tool
    ORDER BY calls DESC
  `).all(...params) as Array<{
    tool: string;
    calls: number;
    success_rate: number;
    avg_duration_ms: number;
    last_called: number;
  }>;

  const tools: ToolStat[] = toolRows.map(r => ({
    name: r.tool,
    calls: r.calls,
    success_rate: Math.round((r.success_rate ?? 0) * 100) / 100,
    avg_duration_ms: Math.round(r.avg_duration_ms ?? 0),
    last_called: r.last_called,
  }));

  const sessionsRow = db
    .prepare(`SELECT COUNT(DISTINCT session_id) as n FROM events WHERE ${postWhere}`)
    .get(...params) as { n: number };

  const projectsRow = db
    .prepare(`SELECT COUNT(DISTINCT project_path) as n FROM events WHERE ${postWhere}`)
    .get(...params) as { n: number };

  const fromTs = opts.all
    ? (db.prepare("SELECT MIN(ts) as t FROM events").get() as { t: number | null })?.t ?? now
    : opts.since
      ? new Date(opts.since).getTime()
      : now - 7 * 24 * 60 * 60 * 1000;

  const result: StatsResult = {
    period: { from: new Date(fromTs).toISOString().slice(0, 10), to: toDate },
    total_calls,
    tools,
    sessions: sessionsRow.n,
    projects: projectsRow.n,
  };

  if (opts.tool && tools.length > 0) {
    result.tool = tools[0];
  }

  return result;
}
