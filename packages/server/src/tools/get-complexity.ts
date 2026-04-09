import { getOrCreateDb } from "@sensei/graph-indexer";
import { loadSenseiConfig } from "@sensei/shared";

export interface ComplexityResult {
  hotspots: Array<{
    name: string;
    file: string;
    line: number;
    complexity: number;
  }>;
  fileScores: Array<{
    file: string;
    maxComplexity: number;
    avgComplexity: number;
    functionCount: number;
  }>;
  summary: {
    totalFunctions: number;
    avgComplexity: number;
    highComplexityCount: number; // complexity > 10
  };
}

export async function getComplexity(
  repoId: string,
  repoPath: string,
  opts: { limit?: number; minComplexity?: number } = {}
): Promise<ComplexityResult> {
  const limit = opts.limit ?? 20;
  const minComplexity = opts.minComplexity ?? 1;

  const config = await loadSenseiConfig(repoPath);
  const project = config?.repo_id ?? repoId;
  const escapedProject = project.replace(/'/g, "\\'");

  const { db, conn } = await getOrCreateDb(repoId);
  try {
    // Get top hotspots
    const hotResult = await conn.query(
      `MATCH (f:Function {project: '${escapedProject}'}) WHERE f.complexity >= ${minComplexity} RETURN f.name AS name, f.file AS file, f.line AS line, f.complexity AS complexity ORDER BY f.complexity DESC LIMIT ${limit}`
    );
    const hotRows = Array.isArray(hotResult) ? hotResult[0] : hotResult;
    const hotData = (await hotRows.getAll()) as Record<string, unknown>[];

    const hotspots = hotData.map(r => ({
      name: String(r.name),
      file: String(r.file).startsWith(repoPath) ? String(r.file).slice(repoPath.length + 1) : String(r.file),
      line: Number(r.line),
      complexity: Number(r.complexity),
    }));

    // Get per-file aggregates
    const fileResult = await conn.query(
      `MATCH (f:Function {project: '${escapedProject}'}) RETURN f.file AS file, MAX(f.complexity) AS maxC, AVG(f.complexity) AS avgC, COUNT(*) AS cnt ORDER BY maxC DESC LIMIT ${limit}`
    );
    const fileRows = Array.isArray(fileResult) ? fileResult[0] : fileResult;
    const fileData = (await fileRows.getAll()) as Record<string, unknown>[];

    const fileScores = fileData.map(r => ({
      file: String(r.file).startsWith(repoPath) ? String(r.file).slice(repoPath.length + 1) : String(r.file),
      maxComplexity: Number(r.maxC),
      avgComplexity: Math.round(Number(r.avgC) * 10) / 10,
      functionCount: Number(r.cnt),
    }));

    // Summary stats
    const sumResult = await conn.query(
      `MATCH (f:Function {project: '${escapedProject}'}) RETURN COUNT(*) AS total, AVG(f.complexity) AS avg, SUM(CASE WHEN f.complexity > 10 THEN 1 ELSE 0 END) AS high`
    );
    const sumRows = Array.isArray(sumResult) ? sumResult[0] : sumResult;
    const sumData = (await sumRows.getAll()) as Record<string, unknown>[];
    const s = sumData[0] ?? {};

    return {
      hotspots,
      fileScores,
      summary: {
        totalFunctions: Number(s.total ?? 0),
        avgComplexity: Math.round(Number(s.avg ?? 1) * 10) / 10,
        highComplexityCount: Number(s.high ?? 0),
      },
    };
  } finally {
    await conn.close();
    await db.close();
  }
}
