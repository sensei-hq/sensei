import { getOrCreateDb, searchSymbols } from "@sensei/graph-indexer";
import { createTokenCounter } from "@sensei/shared";
import { resolveProject } from "./resolve-project.js";
import { relative } from "node:path";

export async function recommendNext(
  repoId: string,
  repoPath: string,
  task: string,
  modelId?: string,
): Promise<{ recommendations: Array<{ filePath: string; score: number; symbolCount: number; estimatedTokens: number }>; suggestedBudget: number }> {
  const counter = createTokenCounter(modelId);

  const project = await resolveProject(repoPath, repoId);

  const { db, conn } = await getOrCreateDb(repoId);
  try {
    const symbols = await searchSymbols(conn, task, project, 100);

    // Group by file (absolute path), aggregate score using 1/(index+1)
    const fileMap = new Map<string, { score: number; syms: typeof symbols }>();
    for (let i = 0; i < symbols.length; i++) {
      const sym = symbols[i];
      if (!sym.file) continue;
      const existing = fileMap.get(sym.file);
      if (existing) {
        existing.score += 1 / (i + 1);
        existing.syms.push(sym);
      } else {
        fileMap.set(sym.file, { score: 1 / (i + 1), syms: [sym] });
      }
    }

    // Sort by score, take top 3
    const top3 = Array.from(fileMap.entries())
      .sort((a, b) => b[1].score - a[1].score)
      .slice(0, 3);

    const recommendations = top3.map(([absPath, { score, syms }]) => {
      const estimatedTokens = syms.reduce(
        (sum, s) => sum + counter.count(`${s.name} ${s.sig ?? ""}`.trim()),
        0
      );
      return {
        filePath: relative(repoPath, absPath),
        score,
        symbolCount: syms.length,
        estimatedTokens,
      };
    });

    const totalEstimated = recommendations.reduce((s, r) => s + r.estimatedTokens, 0);
    const suggestedBudget = Math.min(Math.ceil(totalEstimated * 1.5), 8000);

    return { recommendations, suggestedBudget };
  } finally {
    await conn.close();
    await db.close();
  }
}
