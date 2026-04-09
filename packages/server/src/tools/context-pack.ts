import { getOrCreateDb, searchSymbols } from "@sensei/graph-indexer";
import { loadSenseiConfig, createTokenCounter } from "@sensei/shared";
import { getActivityLog } from "../activity-log.js";
import { readFile } from "node:fs/promises";
import { relative } from "node:path";
import { randomUUID } from "node:crypto";

export interface ContextSlice {
  kind: "code";
  filePath: string;   // repo-relative
  startLine: number;
  endLine: number;
  content: string;
  tokens: number;
  score: number;
  symbolName: string;
}

export async function contextPack(
  repoId: string,
  repoPath: string,
  task: string,
  opts: {
    maxTokens?: number;
    modelId?: string;
    sessionId?: string;
    sessionContext?: string[];
  } = {}
): Promise<{ id: string; task: string; totalTokens: number; modelId: string | null; createdAt: string; slices: ContextSlice[] }> {
  const maxTokens = opts.maxTokens ?? 8000;
  const counter = createTokenCounter(opts.modelId);

  // Get project name from config
  const config = await loadSenseiConfig(repoPath);
  const project = config?.repo_id ?? repoId;

  const { db, conn } = await getOrCreateDb(repoId);
  try {
    const symbols = await searchSymbols(conn, task, project, 100);

    // Group by file, aggregate score using 1/(index+1)
    const fileScores = new Map<string, { score: number; symbols: typeof symbols }>();
    for (let i = 0; i < symbols.length; i++) {
      const sym = symbols[i];
      if (!sym.file) continue;
      // Skip files already in session context
      const relPath = relative(repoPath, sym.file);
      if (opts.sessionContext?.includes(relPath) || opts.sessionContext?.includes(sym.file)) continue;

      const existing = fileScores.get(sym.file);
      if (existing) {
        existing.score += 1 / (i + 1);
        existing.symbols.push(sym);
      } else {
        fileScores.set(sym.file, { score: 1 / (i + 1), symbols: [sym] });
      }
    }

    // Sort files by score descending
    const sortedFiles = Array.from(fileScores.entries()).sort((a, b) => b[1].score - a[1].score);

    const slices: ContextSlice[] = [];
    let totalTokens = 0;

    for (const [absPath, { score, symbols: fileSyms }] of sortedFiles) {
      if (totalTokens >= maxTokens) break;

      let content: string;
      try {
        content = await readFile(absPath, "utf-8");
      } catch {
        continue;
      }

      const lines = content.split("\n");

      // Take up to 3 most relevant symbols per file
      const topSyms = fileSyms.slice(0, 3);
      for (const sym of topSyms) {
        if (totalTokens >= maxTokens) break;

        const startLine = Math.max(0, (sym.line ?? 1) - 1);
        const endLine = Math.min(lines.length - 1, startLine + 20);
        const excerpt = lines.slice(startLine, endLine + 1).join("\n");
        const tokens = counter.count(excerpt);

        if (totalTokens + tokens > maxTokens) break;

        slices.push({
          kind: "code",
          filePath: relative(repoPath, absPath),
          startLine: startLine + 1,
          endLine: endLine + 1,
          content: excerpt,
          tokens,
          score,
          symbolName: sym.name,
        });
        totalTokens += tokens;
      }
    }

    // Log to ActivityLog
    const log = getActivityLog(repoId);
    const packId = log.logContextPack({
      sessionId: opts.sessionId,
      repoId,
      task,
      totalTokens,
      modelId: opts.modelId,
    });

    return {
      id: packId,
      task,
      totalTokens,
      modelId: opts.modelId ?? null,
      createdAt: new Date().toISOString(),
      slices,
    };
  } finally {
    await conn.close();
    await db.close();
  }
}
