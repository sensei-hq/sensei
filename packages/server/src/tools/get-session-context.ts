import { access } from "node:fs/promises";
import { join } from "node:path";
import { getOrCreateDb } from "@sensei/graph-indexer";
import { loadSenseiConfig } from "@sensei/shared";
import { getActivityLog } from "../activity-log.js";
import type { Decision, BacklogItem } from "../activity-log.js";

export interface SolutionContext {
  solutionName: string;
  repos: Array<{ repoId: string; role: string; label?: string }>;
  currentRepoRole: string;
}

export interface SessionContextResult {
  repo_name: string;
  repo_path: string;
  symbol_count: number;
  file_count: number;
  last_indexed_at: string | null;
  stack: string[];
  session_id: string;
  interrupted: Array<{ sessionId: string; startedAt: string; task: string }>;
  memory: {
    decisions: Decision[];
    openBacklog: BacklogItem[];
  };
  solution?: SolutionContext;
  message: string;
}

async function detectStack(repoPath: string): Promise<string[]> {
  const stack: string[] = [];
  const checks: Array<[string, string]> = [
    ["package.json", "typescript"],
    ["pyproject.toml", "python"],
    ["go.mod", "go"],
  ];
  await Promise.all(
    checks.map(async ([file, lang]) => {
      try {
        await access(join(repoPath, file));
        stack.push(lang);
      } catch {
        // file not present
      }
    }),
  );
  return stack;
}

export async function getSessionContext(
  repoId: string,
  repoPath: string,
  sessionId: string,
): Promise<SessionContextResult> {
  const repoName = repoPath.split("/").pop() ?? "repo";

  const [config, stack] = await Promise.all([
    loadSenseiConfig(repoPath),
    detectStack(repoPath),
  ]);

  void config; // loaded for side-effects / future use (custom_libs count)

  let symbolCount = 0;
  let fileCount = 0;

  const { db, conn } = await getOrCreateDb(repoId);
  try {
    const fnResult = await conn.query("MATCH (f:Function) RETURN COUNT(*) AS cnt");
    const fnRows = Array.isArray(fnResult) ? fnResult[0] : fnResult;
    const fnData = await fnRows.getAll();
    symbolCount = Number((fnData[0] as Record<string, unknown>)?.["cnt"] ?? 0);

    const fileResult = await conn.query("MATCH (f:File) RETURN COUNT(*) AS cnt");
    const fileRows = Array.isArray(fileResult) ? fileResult[0] : fileResult;
    const fileData = await fileRows.getAll();
    fileCount = Number((fileData[0] as Record<string, unknown>)?.["cnt"] ?? 0);
  } finally {
    await conn.close();
    await db.close();
  }

  const log = getActivityLog(repoId);
  const recentSessions = log.getRecentSessions(20);
  const interrupted = recentSessions
    .filter((s) => !s.completedAt && s.id !== sessionId)
    .map((s) => ({ sessionId: s.id, startedAt: s.startedAt, task: s.task }));

  const decisions = log.getRecentDecisions(5);
  const openBacklog = log.getOpenBacklog().slice(0, 5);

  // Fetch solution context from the server (posted by the desktop app)
  let solution: SolutionContext | undefined;
  try {
    const port = parseInt(process.env.SENSEI_PORT ?? "7744", 10);
    const res = await fetch(`http://127.0.0.1:${port}/api/solution-context?repoId=${encodeURIComponent(repoId)}`);
    if (res.ok) {
      const data = await res.json() as { solutionName: string; repos: Array<{ repoId: string; role: string; label?: string }> } | null;
      if (data) {
        const currentRepo = data.repos.find(r => r.repoId === repoId);
        solution = {
          solutionName: data.solutionName,
          repos: data.repos.map(r => ({ repoId: r.repoId, role: r.role, label: r.label })),
          currentRepoRole: currentRepo?.role ?? "unknown",
        };
      }
    }
  } catch { /* solution context not available */ }

  const interruptedMsg =
    interrupted.length > 0
      ? ` ${interrupted.length} interrupted session(s) detected — check interrupted[] for recovery context.`
      : "";

  const solutionMsg = solution
    ? ` Part of "${solution.solutionName}" (${solution.repos.length} repos, this repo is ${solution.currentRepoRole}).`
    : "";

  const message = `Repo "${repoName}" — ${symbolCount} functions across ${fileCount} files.${solutionMsg}${interruptedMsg} Use search() to find code, get_symbol(name, depth) for details.`;

  return {
    repo_name: repoName,
    repo_path: repoPath,
    symbol_count: symbolCount,
    file_count: fileCount,
    last_indexed_at: null,
    stack,
    session_id: sessionId,
    interrupted,
    memory: { decisions, openBacklog },
    solution,
    message,
  };
}
