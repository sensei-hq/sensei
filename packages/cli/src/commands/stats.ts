// packages/cli/src/commands/stats.ts
import { makeSenseiClient, loadSenseiConfig } from "@sensei/shared";
import { detectGapPatterns } from "@sensei/collector";

export interface StatsResult {
  period: { from: string; to: string };
  sessions: { total: number; completed: number; abandoned: number; inProgress: number };
  avgFtr: number | null;
  topTools: Array<{ name: string; calls: number; successRate: number; avgDurationMs: number | null }>;
  errorCount: number;
  errorSessions: number;
}

export interface StatsCommandOptions {
  days?: number;
  json?: boolean;
  gaps?: boolean;
  /** Override repo path — used in tests */
  _repoPath?: string;
}

export function buildStatsResult(
  sessions: Array<{ status: string; ftr_score: number | null; id: string }>,
  turns: Array<{ tool: string; success: boolean | null; duration_ms: number | null; task_session_id?: string | null }>,
  period: { from: string; to: string },
): StatsResult {
  const completed = sessions.filter(s => s.status === "completed");
  const abandoned = sessions.filter(s => s.status === "abandoned").length;
  const inProgress = sessions.filter(s => s.status === "in_progress").length;

  const ftrScores = completed.map(s => s.ftr_score).filter((s): s is number => s !== null);
  const avgFtr = ftrScores.length > 0
    ? Math.round((ftrScores.reduce((a, b) => a + b, 0) / ftrScores.length) * 1000) / 1000
    : null;

  // Aggregate turns by tool
  const toolMap = new Map<string, { calls: number; successes: number; totalDuration: number; durationCount: number }>();
  for (const turn of turns) {
    const entry = toolMap.get(turn.tool) ?? { calls: 0, successes: 0, totalDuration: 0, durationCount: 0 };
    entry.calls++;
    if (turn.success === true) entry.successes++;
    if (turn.duration_ms !== null) { entry.totalDuration += turn.duration_ms; entry.durationCount++; }
    toolMap.set(turn.tool, entry);
  }

  const topTools = Array.from(toolMap.entries())
    .map(([name, t]) => ({
      name,
      calls: t.calls,
      successRate: t.calls > 0 ? Math.round((t.successes / t.calls) * 100) / 100 : 0,
      avgDurationMs: t.durationCount > 0 ? Math.round(t.totalDuration / t.durationCount) : null,
    }))
    .sort((a, b) => b.calls - a.calls)
    .slice(0, 10);

  const errorCount = turns.filter(t => t.success === false).length;
  const errorSessionIds = new Set(
    turns
      .filter(t => t.success === false && t.task_session_id != null)
      .map(t => t.task_session_id)
  );

  return {
    period,
    sessions: { total: sessions.length, completed: completed.length, abandoned, inProgress },
    avgFtr,
    topTools,
    errorCount,
    errorSessions: errorSessionIds.size,
  };
}

export function formatStats(result: StatsResult, opts: { json: boolean }): string {
  if (opts.json) return JSON.stringify(result, null, 2);

  const lines: string[] = [];
  const { sessions, avgFtr, topTools, errorCount } = result;

  lines.push(`\nsensei stats — ${result.period.from} → ${result.period.to}`);
  lines.push(`\nSessions  ${sessions.total}   (${sessions.completed} completed, ${sessions.abandoned} abandoned, ${sessions.inProgress} in_progress)`);
  lines.push(`Avg FTR   ${avgFtr !== null ? avgFtr.toFixed(3) : "—"}`);

  if (topTools.length > 0) {
    lines.push(`\nTop tools:`);
    for (const t of topTools) {
      const dur = t.avgDurationMs !== null
        ? t.avgDurationMs >= 1000 ? `${(t.avgDurationMs / 1000).toFixed(1)}s` : `${t.avgDurationMs}ms`
        : "—";
      lines.push(`  ${t.name.padEnd(22)} ${String(t.calls).padStart(4)} calls  ${Math.round(t.successRate * 100)}% success  avg ${dur}`);
    }
  }

  lines.push(`\nErrors    ${errorCount} tool failures across ${result.errorSessions} sessions`);
  return lines.join("\n");
}

export async function stats(opts: StatsCommandOptions): Promise<void> {
  const repoPath = opts._repoPath ?? process.cwd();
  const [client, config] = await Promise.all([
    makeSenseiClient(repoPath),
    loadSenseiConfig(repoPath),
  ]);
  if (!client || !config) {
    console.error("Supabase not configured. Run `sensei init` to configure.");
    return;
  }
  const repoId = config.repo_id;

  const days = opts.days ?? 7;
  const nowMs = Date.now();
  const since = new Date(nowMs - days * 24 * 60 * 60 * 1000).toISOString();
  const from = since.slice(0, 10);
  const to = new Date(nowMs).toISOString().slice(0, 10);

  if (opts.gaps) {
    let query = (client as any).from("events").select("input").eq("tool", "Bash").eq("phase", "pre").not("input", "is", null).gte("ts", since);
    const { data: bashEvents } = await query;
    const commands = (bashEvents ?? [])
      .map((e: any) => (e.input as { command?: string })?.command ?? "")
      .filter(Boolean);
    const gaps = detectGapPatterns(commands);
    if (opts.json) {
      console.log(JSON.stringify({ gaps }, null, 2));
    } else {
      console.log(`\nMissed opportunity report — last ${days} days\n`);
      if (gaps.length === 0) { console.log("  No gaps detected."); return; }
      console.log("Pattern                          Count   Suggested tool");
      console.log("─".repeat(60));
      for (const g of gaps) {
        console.log(`${g.pattern.padEnd(32)} ${String(g.count).padStart(5)}   ${g.suggested_tool}`);
      }
    }
    return;
  }

  const { data: sessions, error: sessionsError } = await (client as any)
    .from("task_sessions")
    .select("id,status,ftr_score")
    .eq("repo_id", repoId)
    .gte("created_at", since);
  if (sessionsError) console.warn("Failed to load task_sessions:", sessionsError.message);

  const { data: turns, error: turnsError } = await (client as any)
    .from("task_turns")
    .select("tool,success,duration_ms,task_session_id")
    .eq("repo_id", repoId)
    .gte("created_at", since);
  if (turnsError) console.warn("Failed to load task_turns:", turnsError.message);

  const result = buildStatsResult(sessions ?? [], turns ?? [], { from, to });
  console.log(formatStats(result, { json: opts.json ?? false }));
}
