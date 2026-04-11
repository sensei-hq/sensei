import { type Connection, type QueryResult, type KuzuValue } from "kuzu";
import { relative } from "node:path";
import { escapeCypherStr } from "./shared.js";

export interface DriftResult {
  file: string;
  line: number;
  comment: string; // the DECISION/WHY comment text
  tag: string;
  issue: string; // human-readable description of drift
}

type KuzuRow = Record<string, KuzuValue>;

async function queryRows(conn: Connection, cypher: string): Promise<KuzuRow[]> {
  const result = await conn.query(cypher);
  const qr = Array.isArray(result) ? (result as QueryResult[])[0] : (result as QueryResult);
  const rows = await qr.getAll();
  if (Array.isArray(result)) {
    for (const r of result as QueryResult[]) r.close();
  } else {
    (result as QueryResult).close();
  }
  return rows;
}

function node(row: KuzuRow, alias: string): KuzuRow { return row[alias] as KuzuRow; }
function str(v: KuzuValue): string { return String(v ?? ""); }
function num(v: KuzuValue): number { return Number(v ?? 0); }

/**
 * Finds functions that have DECISION/WHY comments but whose signatures
 * changed significantly (heuristic: comment mentions a name no longer in sig)
 */
export async function detectDrift(
  conn: Connection,
  project: string
): Promise<DriftResult[]> {
  const eProject = escapeCypherStr(project);
  const driftResults: DriftResult[] = [];

  // Get all DECISION/WHY comments with their annotated functions
  let commentRows: KuzuRow[];
  try {
    commentRows = await queryRows(conn,
      `MATCH (c:Comment)-[:ANNOTATES_FN]->(f:Function {project: '${eProject}'})
       WHERE c.tag = 'DECISION' OR c.tag = 'WHY'
       RETURN c, f`
    );
  } catch {
    return [];
  }

  // Get all function names in the project grouped by file for cross-reference
  let fnRows: KuzuRow[];
  try {
    fnRows = await queryRows(conn,
      `MATCH (f:Function {project: '${eProject}'}) RETURN f.name AS name, f.file AS file`
    );
  } catch {
    return [];
  }

  // Build a set of function names per file
  const fnsByFile = new Map<string, Set<string>>();
  for (const row of fnRows) {
    const name = str(row["name"]);
    const file = str(row["file"]);
    if (!fnsByFile.has(file)) fnsByFile.set(file, new Set());
    fnsByFile.get(file)!.add(name);
  }

  for (const row of commentRows) {
    const c = node(row, "c");
    const f = node(row, "f");

    const commentText = str(c["text"]);
    const commentTag = str(c["tag"]);
    const commentLine = num(c["line"]);
    const commentFile = str(c["file"]);
    const fnName = str(f["name"]);
    const fnSig = str(f["sig"]);

    // Extract meaningful words from the comment (>4 chars, not common stop words)
    const stopWords = new Set([
      "that", "this", "with", "from", "have", "will", "they", "their",
      "there", "before", "after", "always", "never", "because",
      "standard", "handle", "using", "custom",
    ]);

    const words = commentText
      .toLowerCase()
      .split(/[\s,.:;()\[\]{}<>'"!?@#$%^&*=+\-/\\|`~]+/)
      .filter((w) => w.length > 4 && !stopWords.has(w));

    // Check if any word in the comment appears in a function name in the same file
    // but does NOT appear in the current function's signature
    const fileFns = fnsByFile.get(commentFile) ?? new Set<string>();

    for (const word of words) {
      const referencedFns = [...fileFns].filter(
        (fn) => fn !== fnName && fn.toLowerCase().includes(word)
      );

      for (const refFn of referencedFns) {
        if (!fnSig.toLowerCase().includes(word)) {
          driftResults.push({
            file: commentFile,
            line: commentLine,
            comment: commentText,
            tag: commentTag,
            issue: `Comment references '${word}' (possibly '${refFn}') but it's not found in function signature '${fnSig.slice(0, 80)}'`,
          });
          break;
        }
      }
    }
  }

  return driftResults;
}

// ─── Doc drift ────────────────────────────────────────────────────────────────

export interface DocDriftResult {
  /** Relative path of the doc file. */
  docPath: string;
  /** Relative paths of the code files this doc covers that have changed. */
  changedCodeFiles: string[];
}

/**
 * Finds docs that cover code files whose content has changed since a reference
 * set of changed paths.  Uses COVERS edges in the graph.
 *
 * @param changedAbsPaths  Set of absolute paths that changed (e.g. from git diff).
 * @param repoPath         Repo root — used to compute relative paths in results.
 */
export async function detectDocDrift(
  conn: Connection,
  project: string,
  changedAbsPaths: Set<string>,
  repoPath: string
): Promise<DocDriftResult[]> {
  if (changedAbsPaths.size === 0) return [];

  const eProject = escapeCypherStr(project);
  let rows: KuzuRow[];
  try {
    rows = await queryRows(conn,
      `MATCH (d:Doc {project: '${eProject}'})-[:COVERS]->(f:File {project: '${eProject}'})
       RETURN d.path AS docPath, f.path AS filePath`
    );
  } catch {
    return [];
  }

  // Group covered files by doc
  const docMap = new Map<string, string[]>();
  for (const row of rows) {
    const docAbs = str(row["docPath"]);
    const fileAbs = str(row["filePath"]);
    if (!docMap.has(docAbs)) docMap.set(docAbs, []);
    docMap.get(docAbs)!.push(fileAbs);
  }

  const drifted: DocDriftResult[] = [];
  for (const [docAbs, coveredFiles] of docMap) {
    const changedCovered = coveredFiles.filter((f) => changedAbsPaths.has(f));
    if (changedCovered.length > 0) {
      drifted.push({
        docPath: relative(repoPath, docAbs),
        changedCodeFiles: changedCovered.map((f) => relative(repoPath, f)),
      });
    }
  }

  return drifted;
}
