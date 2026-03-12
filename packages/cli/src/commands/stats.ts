import { Database } from "bun:sqlite";
import { existsSync, mkdirSync } from "fs";
import { join } from "path";
import { homedir } from "os";
import { readOrCreateUuid, createTables, queryStats, detectGapPatterns } from "@sensei/collector";
import type { StatsResult } from "@sensei/collector";

export interface StatsCommandOptions {
  all?: boolean;
  tool?: string;
  session?: string;
  since?: string;
  json?: boolean;
  gaps?: boolean;
  /** Override home directory — used in tests to avoid touching ~/.sensei */
  _home?: string;
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
      lines.push(`  [${ts}] ${e.phase} ${e.tool}${e.success === 0 ? " ✗" : ""}`);
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
  const HOME = opts._home ?? homedir();
  const uuidPath = join(HOME, ".sensei", "uuid");
  const uuid = await readOrCreateUuid(uuidPath);
  const dbPath = join(HOME, ".sensei", uuid, "analytics.db");

  if (!existsSync(dbPath)) {
    const parent = join(HOME, ".sensei", uuid);
    mkdirSync(parent, { recursive: true });
  }

  const db = new Database(dbPath);
  createTables(db);

  if (opts.gaps) {
    // Query all Bash commands in the period, honouring --since / --all
    let gapsWhere = "tool = 'Bash' AND phase = 'post' AND input IS NOT NULL";
    const gapsParams: unknown[] = [];
    if (opts.since) {
      gapsWhere += " AND ts >= ?";
      gapsParams.push(new Date(opts.since).getTime());
    } else if (!opts.all) {
      gapsWhere += " AND ts >= ?";
      gapsParams.push(Date.now() - 7 * 24 * 60 * 60 * 1000);
    }
    const bashEvents = db.prepare(`
      SELECT input FROM events WHERE ${gapsWhere}
    `).all(...gapsParams) as Array<{ input: string }>;

    const commands = bashEvents
      .map(e => {
        try {
          const parsed = JSON.parse(e.input) as { command?: string };
          return parsed.command ?? "";
        } catch {
          return e.input;
        }
      })
      .filter(Boolean);

    const gaps = detectGapPatterns(commands);

    if (opts.json) {
      console.log(JSON.stringify({ gaps }, null, 2));
    } else {
      const period = opts.all ? "all time" : "last 7 days";
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

  const result = queryStats(db, {
    all: opts.all,
    tool: opts.tool,
    session: opts.session,
    since: opts.since,
  });

  console.log(formatStats(result, { json: opts.json ?? false }));
}
