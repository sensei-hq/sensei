import { getOrCreateDb, searchSymbols } from "@sensei/graph-indexer";
import { resolveProject } from "./resolve-project.js";

export interface SearchResult {
  symbols: Array<{
    name: string;
    kind: string;
    file_path: string;
    line_start: number;
    signature: string | null;
    docstring: string | null;
  }>;
  total: number;
  query: string;
}

export async function search(
  repoId: string,
  repoPath: string,
  query: string,
  limit = 20,
): Promise<SearchResult> {
  const projectName = await resolveProject(repoPath, repoId);

  const { db, conn } = await getOrCreateDb(repoId);
  try {
    const results = await searchSymbols(conn, query, projectName, limit);
    const symbols = results.map((s) => ({
      name: s.name,
      kind: s.kind,
      file_path: s.file,
      line_start: s.line,
      signature: s.sig ?? null,
      docstring: s.docstring ?? null,
    }));
    return { symbols, total: symbols.length, query };
  } finally {
    await conn.close();
    await db.close();
  }
}
