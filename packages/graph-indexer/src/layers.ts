import { type Connection, type QueryResult, type KuzuValue } from "kuzu";
import { escapeCypherStr } from "./shared.js";

export type Depth = 0 | 1 | 2 | 3 | 4 | 5;

export interface SymbolResult {
  // L0
  name: string;
  kind: "function" | "type" | "file";
  file: string;
  line: number;
  // L1 (depth >= 1)
  sig?: string;
  docstring?: string;
  // L2 (depth >= 2)
  callers?: SymbolRef[];
  callees?: SymbolRef[];
  usedTypes?: SymbolRef[];
  // L3 (depth >= 3)
  imports?: string[]; // files this file imports
  importedBy?: string[]; // files that import this file
  // L4 — reserved for semantic search (not implemented yet)
  // L5 (depth >= 5)
  body?: string;
  comments?: CommentResult[];
}

export interface SymbolRef {
  name: string;
  file: string;
  line: number;
}

export interface CommentResult {
  tag: string;
  text: string;
  line: number;
}

// ─── Query helpers ───────────────────────────────────────────────────────────

type KuzuRow = Record<string, KuzuValue>;

/** Run a Cypher query and return all result rows, properly closing the result handle. */
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

/** Extract a node object from a row, e.g. row["f"] for RETURN f */
function node(row: KuzuRow, alias: string): KuzuRow {
  return row[alias] as KuzuRow;
}

function str(v: KuzuValue): string { return String(v ?? ""); }
function num(v: KuzuValue): number { return Number(v ?? 0); }

function toSymbolRef(n: KuzuRow): SymbolRef {
  return { name: str(n["name"]), file: str(n["file"]), line: num(n["line"]) };
}

function toComment(n: KuzuRow): CommentResult {
  return { tag: str(n["tag"]), text: str(n["text"]), line: num(n["line"]) };
}

// ─── Public API ──────────────────────────────────────────────────────────────

export async function getSymbol(
  conn: Connection,
  name: string,
  project: string,
  depth: Depth
): Promise<SymbolResult | null> {
  const eName = escapeCypherStr(name);
  const eProject = escapeCypherStr(project);

  // Try function first
  let fnRows: KuzuRow[] = [];
  try {
    fnRows = await queryRows(conn,
      `MATCH (f:Function {name: '${eName}', project: '${eProject}'}) RETURN f`
    );
  } catch { /* ignore */ }

  if (fnRows.length > 0) {
    const fn = node(fnRows[0], "f");
    const base: SymbolResult = {
      name: str(fn["name"]),
      kind: "function",
      file: str(fn["file"]),
      line: num(fn["line"]),
    };

    if (depth >= 1) {
      base.sig = str(fn["sig"]);
      base.docstring = str(fn["docstring"]);
    }

    if (depth >= 2) {
      base.callers = await getCallers(conn, name, project, 1);

      try {
        const calleeRows = await queryRows(conn,
          `MATCH (f:Function {name: '${eName}', project: '${eProject}'})-[:CALLS]->(g:Function) RETURN g`
        );
        base.callees = calleeRows.map(r => toSymbolRef(node(r, "g")));
      } catch {
        base.callees = [];
      }

      // usedTypes — USES_TYPE edges are not yet populated by the indexer
      base.usedTypes = [];
    }

    if (depth >= 3) {
      const fileId = `file:${str(fn["file"])}`;
      const eFileId = escapeCypherStr(fileId);

      try {
        const importRows = await queryRows(conn,
          `MATCH (f:File {id: '${eFileId}'})-[:IMPORTS]->(g:File) RETURN g.path AS path`
        );
        base.imports = importRows.map(r => str(r["path"]));
      } catch {
        base.imports = [];
      }

      try {
        const importedByRows = await queryRows(conn,
          `MATCH (g:File)-[:IMPORTS]->(f:File {id: '${eFileId}'}) RETURN g.path AS path`
        );
        base.importedBy = importedByRows.map(r => str(r["path"]));
      } catch {
        base.importedBy = [];
      }
    }

    if (depth >= 5) {
      base.body = str(fn["body"]);

      try {
        const commentRows = await queryRows(conn,
          `MATCH (c:Comment)-[:ANNOTATES_FN]->(f:Function {name: '${eName}', project: '${eProject}'}) RETURN c`
        );
        base.comments = commentRows.map(r => toComment(node(r, "c")));
      } catch {
        base.comments = [];
      }
    }

    return base;
  }

  // Try type
  let typeRows: KuzuRow[] = [];
  try {
    typeRows = await queryRows(conn,
      `MATCH (t:Type {name: '${eName}', project: '${eProject}'}) RETURN t`
    );
  } catch { /* ignore */ }

  if (typeRows.length > 0) {
    const t = node(typeRows[0], "t");
    const base: SymbolResult = {
      name: str(t["name"]),
      kind: "type",
      file: str(t["file"]),
      line: num(t["line"]),
    };

    if (depth >= 1) {
      base.sig = str(t["kind"]);
    }

    if (depth >= 3) {
      const fileId = `file:${str(t["file"])}`;
      const eFileId = escapeCypherStr(fileId);

      try {
        const importRows = await queryRows(conn,
          `MATCH (f:File {id: '${eFileId}'})-[:IMPORTS]->(g:File) RETURN g.path AS path`
        );
        base.imports = importRows.map(r => str(r["path"]));
      } catch {
        base.imports = [];
      }
    }

    if (depth >= 5) {
      try {
        const commentRows = await queryRows(conn,
          `MATCH (c:Comment)-[:ANNOTATES_TYPE]->(t:Type {name: '${eName}', project: '${eProject}'}) RETURN c`
        );
        base.comments = commentRows.map(r => toComment(node(r, "c")));
      } catch {
        base.comments = [];
      }
    }

    return base;
  }

  return null;
}

export async function getCallers(
  conn: Connection,
  name: string,
  project: string,
  hops: number
): Promise<SymbolRef[]> {
  const eName = escapeCypherStr(name);
  const eProject = escapeCypherStr(project);

  try {
    const rows = await queryRows(conn,
      `MATCH (g:Function)-[:CALLS]->(f:Function {name: '${eName}', project: '${eProject}'}) RETURN g`
    );
    return rows.map(r => toSymbolRef(node(r, "g")));
  } catch {
    return [];
  }
}

export async function searchSymbols(
  conn: Connection,
  query: string,
  project: string,
  limit: number
): Promise<SymbolResult[]> {
  const eProject = escapeCypherStr(project);
  const eQuery = escapeCypherStr(query);
  const results: SymbolResult[] = [];

  // Search functions
  try {
    const fnRows = await queryRows(conn,
      `MATCH (f:Function {project: '${eProject}'}) WHERE f.name CONTAINS '${eQuery}' RETURN f LIMIT ${limit}`
    );
    for (const row of fnRows) {
      const f = node(row, "f");
      results.push({
        name: str(f["name"]),
        kind: "function",
        file: str(f["file"]),
        line: num(f["line"]),
        sig: str(f["sig"]),
        docstring: str(f["docstring"]),
      });
    }
  } catch { /* ignore */ }

  // Search types
  if (results.length < limit) {
    try {
      const typeRows = await queryRows(conn,
        `MATCH (t:Type {project: '${eProject}'}) WHERE t.name CONTAINS '${eQuery}' RETURN t LIMIT ${limit - results.length}`
      );
      for (const row of typeRows) {
        const t = node(row, "t");
        results.push({
          name: str(t["name"]),
          kind: "type",
          file: str(t["file"]),
          line: num(t["line"]),
        });
      }
    } catch { /* ignore */ }
  }

  return results;
}
