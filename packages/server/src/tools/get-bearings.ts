import { McpServer } from "@modelcontextprotocol/sdk/server/mcp.js";
import { z } from "zod";
import { getOrCreateDb } from "@sensei/graph-indexer";
import { loadSenseiConfig } from "@sensei/shared";
import { getActivityLog } from "../activity-log.js";
import type { McpServerOptions } from "../mcp-server.js";

function formatDate(iso: string): string {
  try {
    const d = new Date(iso);
    return d.toLocaleDateString("en-US", { month: "short", day: "numeric" });
  } catch {
    return iso.slice(0, 10);
  }
}

async function getGraphStats(repoId: string): Promise<{ functions: number; files: number }> {
  try {
    const { conn } = await getOrCreateDb(repoId);
    try {
      const fnResult = await conn.query("MATCH (f:Function) RETURN COUNT(*) AS cnt");
      const fnRows = await (Array.isArray(fnResult) ? fnResult[0] : fnResult).getAll();
      const fnCount = Number(fnRows[0]?.["cnt"] ?? 0);

      const fileResult = await conn.query("MATCH (f:File) RETURN COUNT(*) AS cnt");
      const fileRows = await (Array.isArray(fileResult) ? fileResult[0] : fileResult).getAll();
      const fileCount = Number(fileRows[0]?.["cnt"] ?? 0);

      return { functions: fnCount, files: fileCount };
    } finally {
      await conn.close();
    }
  } catch {
    return { functions: 0, files: 0 };
  }
}

export function registerGetBearingsTool(server: McpServer, opts: McpServerOptions): void {
  server.tool(
    "get_bearings",
    "Get a compact briefing to restore ACP context: recent sessions, open backlog, recent decisions, and graph stats",
    {
      detail: z
        .boolean()
        .optional()
        .default(false)
        .describe("Include extra detail (more sessions/decisions)"),
    },
    async ({ detail }) => {
      try {
        const limit = detail ? 10 : 5;
        const activityLog = getActivityLog(opts.repoId);
        const config = await loadSenseiConfig(opts.repoPath);
        const repoName = config?.repo_id ?? opts.repoId;

        const [graphStats, sessions, backlog, decisions] = await Promise.all([
          getGraphStats(opts.repoId),
          Promise.resolve(activityLog.getRecentSessions(limit)),
          Promise.resolve(activityLog.getOpenBacklog()),
          Promise.resolve(activityLog.getRecentDecisions(limit)),
        ]);

        const lines: string[] = [];

        lines.push(`## Repo: ${repoName} (${opts.repoId})`);
        lines.push(
          `## Graph: ${graphStats.functions} functions, ${graphStats.files} files indexed`
        );

        lines.push(`\n## Recent Sessions (last ${Math.min(limit, sessions.length)})`);
        if (sessions.length === 0) {
          lines.push("  (none yet)");
        } else {
          sessions.forEach((s, i) => {
            const outcome = s.outcome ?? "active";
            const date = formatDate(s.startedAt);
            lines.push(`${i + 1}. [${outcome}] ${s.task} — ${date}`);
          });
        }

        const topBacklog = backlog.slice(0, limit);
        lines.push(`\n## Open Backlog (${backlog.length} items, showing top ${topBacklog.length})`);
        if (topBacklog.length === 0) {
          lines.push("  (none)");
        } else {
          for (const item of topBacklog) {
            lines.push(`- [${item.priority}] ${item.title}`);
          }
        }

        lines.push(`\n## Recent Decisions`);
        if (decisions.length === 0) {
          lines.push("  (none recorded)");
        } else {
          for (const d of decisions) {
            lines.push(`- ${formatDate(d.timestamp)}: ${d.text}`);
          }
        }

        lines.push(`\n## ACP Context`);
        lines.push(
          `Repo is a TypeScript monorepo. Use get_symbol(name, depth) to look up code. Use search_code_graph(query) to find symbols. Use index_repo() to trigger a fresh index.`
        );

        return { content: [{ type: "text", text: lines.join("\n") }] };
      } catch (err) {
        return {
          content: [{ type: "text", text: `Error: ${err instanceof Error ? err.message : String(err)}` }],
          isError: true,
        };
      }
    }
  );
}
