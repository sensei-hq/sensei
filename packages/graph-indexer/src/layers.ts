import { type Connection, type QueryResult, type KuzuValue } from "kuzu";

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

export async function getSymbol(
  conn: Connection,
  name: string,
  project: string,
  depth: Depth
): Promise<SymbolResult | null> {
  const escapedName = escapeCypher(name);
  const escapedProject = escapeCypher(project);

  // Try function first
  let fnRows: Record<string, KuzuValue>[] = [];
  try {
    const result = await conn.query(
      `MATCH (f:Function {name: '${escapedName}', project: '${escapedProject}'}) RETURN f`
    );
    fnRows = await getAllRows(result as QueryResult | QueryResult[]);
  } catch {
    // ignore
  }

  if (fnRows.length > 0) {
    const fn = fnRows[0]["f"] as Record<string, KuzuValue>;
    const base: SymbolResult = {
      name: String(fn["name"] ?? ""),
      kind: "function",
      file: String(fn["file"] ?? ""),
      line: Number(fn["line"] ?? 0),
    };

    if (depth >= 1) {
      base.sig = String(fn["sig"] ?? "");
      base.docstring = String(fn["docstring"] ?? "");
    }

    if (depth >= 2) {
      // callers: functions that CALL this function
      base.callers = await getCallers(conn, name, project, 1);

      // callees: functions this function calls
      try {
        const calleeResult = await conn.query(
          `MATCH (f:Function {name: '${escapedName}', project: '${escapedProject}'})-[:CALLS]->(g:Function) RETURN g`
        );
        const calleeRows = await getAllRows(
          calleeResult as QueryResult | QueryResult[]
        );
        base.callees = calleeRows.map((r) => {
          const g = r["g"] as Record<string, KuzuValue>;
          return {
            name: String(g["name"] ?? ""),
            file: String(g["file"] ?? ""),
            line: Number(g["line"] ?? 0),
          };
        });
      } catch {
        base.callees = [];
      }

      // usedTypes
      try {
        const typeResult = await conn.query(
          `MATCH (f:Function {name: '${escapedName}', project: '${escapedProject}'})-[:USES_TYPE]->(t:Type) RETURN t`
        );
        const typeRows = await getAllRows(
          typeResult as QueryResult | QueryResult[]
        );
        base.usedTypes = typeRows.map((r) => {
          const t = r["t"] as Record<string, KuzuValue>;
          return {
            name: String(t["name"] ?? ""),
            file: String(t["file"] ?? ""),
            line: Number(t["line"] ?? 0),
          };
        });
      } catch {
        base.usedTypes = [];
      }
    }

    if (depth >= 3) {
      // Find the file this function belongs to
      const fnFile = String(fn["file"] ?? "");
      const fileId = `file:${fnFile}`;
      const escapedFileId = escapeCypher(fileId);

      // imports
      try {
        const importResult = await conn.query(
          `MATCH (f:File {id: '${escapedFileId}'})-[:IMPORTS]->(g:File) RETURN g.path AS path`
        );
        const importRows = await getAllRows(
          importResult as QueryResult | QueryResult[]
        );
        base.imports = importRows.map((r) => String(r["path"] ?? ""));
      } catch {
        base.imports = [];
      }

      // importedBy
      try {
        const importedByResult = await conn.query(
          `MATCH (g:File)-[:IMPORTS]->(f:File {id: '${escapedFileId}'}) RETURN g.path AS path`
        );
        const importedByRows = await getAllRows(
          importedByResult as QueryResult | QueryResult[]
        );
        base.importedBy = importedByRows.map((r) => String(r["path"] ?? ""));
      } catch {
        base.importedBy = [];
      }
    }

    if (depth >= 5) {
      base.body = String(fn["body"] ?? "");

      // comments annotating this function
      try {
        const commentResult = await conn.query(
          `MATCH (c:Comment)-[:ANNOTATES_FN]->(f:Function {name: '${escapedName}', project: '${escapedProject}'}) RETURN c`
        );
        const commentRows = await getAllRows(
          commentResult as QueryResult | QueryResult[]
        );
        base.comments = commentRows.map((r) => {
          const c = r["c"] as Record<string, KuzuValue>;
          return {
            tag: String(c["tag"] ?? ""),
            text: String(c["text"] ?? ""),
            line: Number(c["line"] ?? 0),
          };
        });
      } catch {
        base.comments = [];
      }
    }

    return base;
  }

  // Try type
  let typeRows: Record<string, KuzuValue>[] = [];
  try {
    const result = await conn.query(
      `MATCH (t:Type {name: '${escapedName}', project: '${escapedProject}'}) RETURN t`
    );
    typeRows = await getAllRows(result as QueryResult | QueryResult[]);
  } catch {
    // ignore
  }

  if (typeRows.length > 0) {
    const t = typeRows[0]["t"] as Record<string, KuzuValue>;
    const base: SymbolResult = {
      name: String(t["name"] ?? ""),
      kind: "type",
      file: String(t["file"] ?? ""),
      line: Number(t["line"] ?? 0),
    };

    if (depth >= 1) {
      base.sig = String(t["kind"] ?? ""); // use kind as sig for types
    }

    if (depth >= 3) {
      const typeFile = String(t["file"] ?? "");
      const fileId = `file:${typeFile}`;
      const escapedFileId = escapeCypher(fileId);

      try {
        const importResult = await conn.query(
          `MATCH (f:File {id: '${escapedFileId}'})-[:IMPORTS]->(g:File) RETURN g.path AS path`
        );
        const importRows = await getAllRows(
          importResult as QueryResult | QueryResult[]
        );
        base.imports = importRows.map((r) => String(r["path"] ?? ""));
      } catch {
        base.imports = [];
      }
    }

    if (depth >= 5) {
      // comments annotating this type
      try {
        const commentResult = await conn.query(
          `MATCH (c:Comment)-[:ANNOTATES_TYPE]->(t:Type {name: '${escapedName}', project: '${escapedProject}'}) RETURN c`
        );
        const commentRows = await getAllRows(
          commentResult as QueryResult | QueryResult[]
        );
        base.comments = commentRows.map((r) => {
          const c = r["c"] as Record<string, KuzuValue>;
          return {
            tag: String(c["tag"] ?? ""),
            text: String(c["text"] ?? ""),
            line: Number(c["line"] ?? 0),
          };
        });
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
  const escapedName = escapeCypher(name);
  const escapedProject = escapeCypher(project);

  // Simple 1-hop for now; variable hops could use Kuzu's recursive patterns
  try {
    const result = await conn.query(
      `MATCH (g:Function)-[:CALLS]->(f:Function {name: '${escapedName}', project: '${escapedProject}'}) RETURN g`
    );
    const rows = await getAllRows(result as QueryResult | QueryResult[]);
    return rows.map((r) => {
      const g = r["g"] as Record<string, KuzuValue>;
      return {
        name: String(g["name"] ?? ""),
        file: String(g["file"] ?? ""),
        line: Number(g["line"] ?? 0),
      };
    });
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
  const escapedProject = escapeCypher(project);
  const results: SymbolResult[] = [];

  // Search functions
  try {
    const fnResult = await conn.query(
      `MATCH (f:Function {project: '${escapedProject}'}) WHERE f.name CONTAINS '${escapeCypher(query)}' RETURN f LIMIT ${limit}`
    );
    const fnRows = await getAllRows(fnResult as QueryResult | QueryResult[]);
    for (const row of fnRows) {
      const f = row["f"] as Record<string, KuzuValue>;
      results.push({
        name: String(f["name"] ?? ""),
        kind: "function",
        file: String(f["file"] ?? ""),
        line: Number(f["line"] ?? 0),
        sig: String(f["sig"] ?? ""),
        docstring: String(f["docstring"] ?? ""),
      });
    }
  } catch {
    // ignore
  }

  // Search types
  if (results.length < limit) {
    try {
      const typeResult = await conn.query(
        `MATCH (t:Type {project: '${escapedProject}'}) WHERE t.name CONTAINS '${escapeCypher(query)}' RETURN t LIMIT ${limit - results.length}`
      );
      const typeRows = await getAllRows(
        typeResult as QueryResult | QueryResult[]
      );
      for (const row of typeRows) {
        const t = row["t"] as Record<string, KuzuValue>;
        results.push({
          name: String(t["name"] ?? ""),
          kind: "type",
          file: String(t["file"] ?? ""),
          line: Number(t["line"] ?? 0),
        });
      }
    } catch {
      // ignore
    }
  }

  return results;
}
