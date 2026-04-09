import { readFile } from "fs/promises";
import { join } from "path";
import { getOrCreateDb } from "@sensei/graph-indexer";
import { loadSenseiConfig } from "@sensei/shared";

export interface LoadContextResult {
  file_path: string;
  content: string;
  symbols: Array<{
    name: string;
    kind: string;
    line_start: number;
    signature: string | null;
  }>;
  line_count: number;
}

export async function loadContext(
  repoId: string,
  repoPath: string,
  filePath: string,
): Promise<LoadContextResult> {
  const absPath = join(repoPath, filePath);
  let content: string;
  try {
    content = await readFile(absPath, "utf-8");
  } catch {
    throw new Error(`File not found: ${filePath}`);
  }

  const config = await loadSenseiConfig(repoPath);
  const project = config?.repo_id ?? repoId;
  const escapedFile = absPath.replace(/'/g, "\\'");
  const escapedProject = project.replace(/'/g, "\\'");

  const symbols: LoadContextResult["symbols"] = [];
  const { db, conn } = await getOrCreateDb(repoId);
  try {
    const fnResult = await conn.query(
      `MATCH (n:Function {file: '${escapedFile}', project: '${escapedProject}'}) RETURN n.name AS name, n.line AS line, n.sig AS sig ORDER BY n.line`
    );
    const fnRows = Array.isArray(fnResult) ? fnResult[0] : fnResult;
    for (const row of (await fnRows.getAll()) as Record<string, unknown>[]) {
      symbols.push({ name: String(row.name), kind: "function", line_start: Number(row.line), signature: (row.sig as string) || null });
    }

    const typeResult = await conn.query(
      `MATCH (n:Type {file: '${escapedFile}', project: '${escapedProject}'}) RETURN n.name AS name, n.line AS line ORDER BY n.line`
    );
    const typeRows = Array.isArray(typeResult) ? typeResult[0] : typeResult;
    for (const row of (await typeRows.getAll()) as Record<string, unknown>[]) {
      symbols.push({ name: String(row.name), kind: "type", line_start: Number(row.line), signature: null });
    }
  } finally {
    await conn.close();
    await db.close();
  }

  symbols.sort((a, b) => a.line_start - b.line_start);

  return {
    file_path: filePath,
    content,
    symbols,
    line_count: content.split("\n").length,
  };
}
