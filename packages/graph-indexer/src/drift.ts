import { type Connection, type QueryResult, type KuzuValue } from "kuzu";

export interface DriftResult {
  file: string;
  line: number;
  comment: string; // the DECISION/WHY comment text
  tag: string;
  issue: string; // human-readable description of drift
}

async function getAllRows(
  result: QueryResult | QueryResult[]
): Promise<Record<string, KuzuValue>[]> {
  const qr = Array.isArray(result) ? result[0] : result;
  const rows = await qr.getAll();
  if (Array.isArray(result)) {
    for (const r of result) r.close();
  } else {
    result.close();
  }
  return rows;
}

function escapeCypher(s: string): string {
  return s.replace(/\\/g, "\\\\").replace(/'/g, "\\'");
}

/**
 * Finds functions that have DECISION/WHY comments but whose signatures
 * changed significantly (heuristic: comment mentions a name no longer in sig)
 */
export async function detectDrift(
  conn: Connection,
  project: string
): Promise<DriftResult[]> {
  const escapedProject = escapeCypher(project);
  const driftResults: DriftResult[] = [];

  // Get all DECISION/WHY comments with their annotated functions
  let commentRows: Record<string, KuzuValue>[] = [];
  try {
    const result = await conn.query(
      `MATCH (c:Comment)-[:ANNOTATES_FN]->(f:Function {project: '${escapedProject}'})
       WHERE c.tag = 'DECISION' OR c.tag = 'WHY'
       RETURN c, f`
    );
    commentRows = await getAllRows(result as QueryResult | QueryResult[]);
  } catch {
    return [];
  }

  // Get all function names in the project grouped by file for cross-reference
  let fnRows: Record<string, KuzuValue>[] = [];
  try {
    const result = await conn.query(
      `MATCH (f:Function {project: '${escapedProject}'}) RETURN f.name AS name, f.file AS file`
    );
    fnRows = await getAllRows(result as QueryResult | QueryResult[]);
  } catch {
    return [];
  }

  // Build a set of function names per file
  const fnsByFile = new Map<string, Set<string>>();
  for (const row of fnRows) {
    const name = String(row["name"] ?? "");
    const file = String(row["file"] ?? "");
    if (!fnsByFile.has(file)) fnsByFile.set(file, new Set());
    fnsByFile.get(file)!.add(name);
  }

  for (const row of commentRows) {
    const c = row["c"] as Record<string, KuzuValue>;
    const f = row["f"] as Record<string, KuzuValue>;

    const commentText = String(c["text"] ?? "");
    const commentTag = String(c["tag"] ?? "");
    const commentLine = Number(c["line"] ?? 0);
    const commentFile = String(c["file"] ?? "");
    const fnName = String(f["name"] ?? "");
    const fnSig = String(f["sig"] ?? "");

    // Extract meaningful words from the comment (>4 chars, not common stop words)
    const stopWords = new Set([
      "that",
      "this",
      "with",
      "from",
      "have",
      "will",
      "they",
      "their",
      "there",
      "before",
      "after",
      "always",
      "never",
      "because",
      "standard",
      "handle",
      "using",
      "custom",
    ]);

    const words = commentText
      .toLowerCase()
      .split(/[\s,.:;()\[\]{}<>'"!?@#$%^&*=+\-/\\|`~]+/)
      .filter((w) => w.length > 4 && !stopWords.has(w));

    // Check if any word in the comment appears in a function name in the same file
    // but does NOT appear in the current function's signature
    const fileFns = fnsByFile.get(commentFile) ?? new Set<string>();

    for (const word of words) {
      // Does this word reference another function name in the file?
      const referencedFns = [...fileFns].filter(
        (fn) => fn !== fnName && fn.toLowerCase().includes(word)
      );

      for (const refFn of referencedFns) {
        // Check if the referenced function name is mentioned in the current sig
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
