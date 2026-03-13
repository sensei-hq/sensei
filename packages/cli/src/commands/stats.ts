import { makeSenseiClient } from "@sensei/shared";
import { queryStats, detectGapPatterns } from "@sensei/collector";
import type { StatsResult } from "@sensei/collector";

export interface StatsCommandOptions {
  all?: boolean;
  tool?: string;
  session?: string;
  since?: string;
  json?: boolean;
  gaps?: boolean;
  /** Override repo path — used in tests */
  _repoPath?: string;
}

export function formatStats(result: StatsResult, opts: { json: boolean }): string {
  if (opts.json) {
    return JSON.stringify(result, null, 2);
  }

  const lines: string[] = [];

  if (result.tool) {
    // Single-tool view
    const t = result.tool;
    lines.push(`\nsensei stats — ${t.name}`);
    lines.push(`  Calls:       ${t.calls}`);
    lines.push(`  Success:     ${Math.round(t.success_rate * 100)}%`);
    lines.push(`  Avg latency: ${t.avg_duration_ms}ms`);
    if (t.last_called) {
      lines.push(`  Last called: ${new Date(t.last_called).toISOString()}`);
    }
    return lines.join("\n");
  }

  if (result.events) {
    // Session view
    lines.push(`\nSession events (${result.events.length}):`);
    for (const e of result.events) {
      const ts = new Date((e.ts as number) ?? 0).toISOString().slice(11, 19);
      lines.push(`  [${ts}] ${e.phase} ${e.tool}${e.success === false ? " ✗" : ""}`);
    }
    return lines.join("\n");
  }

  // Default summary
  lines.push(`\nsensei stats — ${result.period.from} → ${result.period.to}`);
  lines.push(`\nTool calls: ${result.total_calls}`);

  const top5 = result.tools.slice(0, 5);
  for (const t of top5) {
    const pct = result.total_calls > 0
      ? Math.round((t.calls / result.total_calls) * 100)
      : 0;
    const successStr = `✓ ${Math.round(t.success_rate * 100)}%`;
    const durStr = t.avg_duration_ms >= 1000
      ? `${(t.avg_duration_ms / 1000).toFixed(1)}s`
      : `${t.avg_duration_ms}ms`;
    lines.push(`  ${t.name.padEnd(20)} ${String(t.calls).padStart(4)}  (${String(pct).padStart(2)}%)  ${successStr}  avg ${durStr}`);
  }
  if (result.tools.length > 5) lines.push(`  (top 5 by call count)`);
  lines.push(`\nSessions: ${result.sessions} across ${result.projects} project${result.projects !== 1 ? "s" : ""}`);

  return lines.join("\n");
}

export async function stats(opts: StatsCommandOptions): Promise<void> {
  const client = await makeSenseiClient(opts._repoPath ?? process.cwd());
  if (!client) {
    console.error("Supabase not configured. Run `sensei setup` to configure.");
    return;
  }

  if (opts.gaps) {
    const since = opts.all ? undefined : opts.since
      ? new Date(opts.since).toISOString()
      : new Date(Date.now() - 7 * 24 * 60 * 60 * 1000).toISOString();

    let query = client.from("events").select("input").eq("tool", "Bash").eq("phase", "pre").not("input", "is", null);
    if (since) query = query.gte("ts", since);
    const { data: bashEvents } = await query;

    const commands = (bashEvents ?? [])
      .map((e: any) => (e.input as { command?: string })?.command ?? "")
      .filter(Boolean);

    const gaps = detectGapPatterns(commands);

    if (opts.json) {
      console.log(JSON.stringify({ gaps }, null, 2));
    } else {
      const period = opts.all ? "all time" : opts.since ? `since ${opts.since}` : "last 7 days";
      console.log(`\nMissed opportunity report — ${period}\n`);
      if (gaps.length === 0) {
        console.log("  No gaps detected.");
        return;
      }
      const header = "Pattern                          Count   Suggested tool";
      const sep    = "─".repeat(header.length);
      console.log(header);
      console.log(sep);
      for (const g of gaps) {
        console.log(`${g.pattern.padEnd(32)} ${String(g.count).padStart(5)}   ${g.suggested_tool}`);
      }
    }
    return;
  }

  const result = await queryStats(client, {
    all: opts.all,
    tool: opts.tool,
    session: opts.session,
    since: opts.since,
  });
  console.log(formatStats(result, { json: opts.json ?? false }));
}
