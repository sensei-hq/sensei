import {
  type Database,
  type Connection,
  type QueryResult,
  type KuzuValue,
} from "kuzu";
import fg from "fast-glob";
import { readFile, mkdir, writeFile } from "node:fs/promises";
import { join, resolve, dirname, relative } from "node:path";
import { homedir } from "node:os";
import { existsSync } from "node:fs";
import { execFileSync } from "node:child_process";
import { TypeScriptAdapter } from "@sensei/engine";
import { extractTaggedComments } from "./parser.js";
import { ensureSchemaWithConn } from "./schema.js";

const FUNCTION_KINDS = new Set(["function", "method", "component", "hook"]);

function isFunctionLike(kind: string): boolean {
  return FUNCTION_KINDS.has(kind);
}

export interface IndexOptions {
  repoPath: string;
  repoId: string;
  project: string; // display name
  include?: string[]; // glob patterns, default ['**/*.ts', '**/*.tsx']
  exclude?: string[]; // default ['**/node_modules/**', '**/dist/**', '**/*.spec.ts', '**/*.d.ts']
}

export interface IndexResult {
  filesIndexed: number;
  functionsIndexed: number;
  typesIndexed: number;
  edgesCreated: number;
  durationMs: number;
}

export async function getOrCreateDb(
  repoId: string
): Promise<{ db: Database; conn: Connection }> {
  const dbDir = join(homedir(), ".sensei", "projects", repoId);
  if (!existsSync(dbDir)) {
    await mkdir(dbDir, { recursive: true });
  }
  const dbPath = join(dbDir, "graph.kuzu");
  const kuzu = await import("kuzu");
  const db = new kuzu.Database(dbPath);
  const conn = new kuzu.Connection(db);
  return { db, conn };
}

async function runQuery(
  conn: Connection,
  statement: string
): Promise<QueryResult | QueryResult[]> {
  return conn.query(statement) as Promise<QueryResult | QueryResult[]>;
}

function closeResult(result: QueryResult | QueryResult[]): void {
  if (Array.isArray(result)) {
    for (const r of result) r.close();
  } else {
    result.close();
  }
}

function escapeCypherStr(s: string): string {
  return s
    .replace(/\\/g, "\\\\")
    .replace(/'/g, "\\'")
    .replace(/\n/g, "\\n")
    .replace(/\r/g, "\\r")
    .replace(/\t/g, "\\t");
}

async function mergeNode(
  conn: Connection,
  label: string,
  props: Record<string, string | number>
): Promise<void> {
  const id = props["id"] as string;
  const setParts = Object.entries(props)
    .filter(([k]) => k !== "id")
    .map(([k, v]) =>
      typeof v === "number"
        ? `n.${k} = ${v}`
        : `n.${k} = '${escapeCypherStr(String(v))}'`
    )
    .join(", ");

  const cypher = `
    MERGE (n:${label} {id: '${escapeCypherStr(id)}'})
    SET ${setParts}
  `;
  const r = await runQuery(conn, cypher);
  closeResult(r);
}

async function mergeEdge(
  conn: Connection,
  fromLabel: string,
  fromId: string,
  toLabel: string,
  toId: string,
  relType: string,
  props?: Record<string, string | number>
): Promise<void> {
  const setPart =
    props
      ? " SET e." +
        Object.entries(props)
          .map(
            ([k, v]) =>
              `${k} = ${
                typeof v === "number"
                  ? v
                  : `'${escapeCypherStr(String(v))}'`
              }`
          )
          .join(", e.")
      : "";

  const cypher = `
    MATCH (a:${fromLabel} {id: '${escapeCypherStr(fromId)}'})
    MATCH (b:${toLabel} {id: '${escapeCypherStr(toId)}'})
    MERGE (a)-[e:${relType}]->(b)${setPart}
  `;
  try {
    const r = await runQuery(conn, cypher);
    closeResult(r);
  } catch {
    // Edge may fail if nodes don't exist yet — skip silently
  }
}

export interface SingleFileResult {
  functionsIndexed: number;
  typesIndexed: number;
  edgesCreated: number;
  imports: { targetPath: string; names: string[] }[];
  callEdges: { callerName: string; calleeName: string; callerId: string }[];
}

/**
 * Delete all graph nodes (and their edges) that belong to a given file.
 * Used before re-indexing a changed file.
 */
export async function deleteFileFromGraph(
  conn: Connection,
  absPath: string,
  project: string
): Promise<void> {
  const escaped = escapeCypherStr(absPath);
  const escapedProject = escapeCypherStr(project);

  for (const label of ["Function", "Type", "Comment"] as const) {
    try {
      const r = await runQuery(
        conn,
        `MATCH (n:${label} {file: '${escaped}', project: '${escapedProject}'}) DETACH DELETE n`
      );
      closeResult(r);
    } catch {
      // ignore — table may be empty
    }
  }
  try {
    const r = await runQuery(
      conn,
      `MATCH (f:File {id: 'file:${escaped}'}) DETACH DELETE f`
    );
    closeResult(r);
  } catch {}
}

function computeComplexity(body: string): number {
  // Approximate cyclomatic complexity: 1 + count of branching constructs
  const patterns = [
    /\bif\b/g, /\bfor\b/g, /\bwhile\b/g, /\bdo\b/g,
    /\bcase\b/g, /\bcatch\b/g, /&&/g, /\|\|/g,
    /\?\s*[^?:]/g, // ternary (not nullish coalescing ??)
  ];
  let n = 1;
  for (const p of patterns) n += (body.match(p) ?? []).length;
  return n;
}

/**
 * Index a single file into an open Kuzu connection.
 * Does NOT handle cross-file CALLS/IMPORTS edges — callers should do a
 * follow-up pass using the returned `imports` and `callEdges`.
 */
export async function indexSingleFile(
  conn: Connection,
  absPath: string,
  repoPath: string,
  project: string,
  adapter: TypeScriptAdapter
): Promise<SingleFileResult> {
  const relPath = relative(repoPath, absPath);
  const parsed = await adapter.parse({ path: relPath, absPath, mtime: 0, hash: "", size: 0 });
  const fileContent = await readFile(absPath, "utf-8");

  const fileId = `file:${absPath}`;
  const moduleName = relPath.replace(/\.(ts|tsx)$/, "").replace(/\\/g, "/");

  await mergeNode(conn, "File", {
    id: fileId,
    path: absPath,
    module: moduleName,
    lang: parsed.language,
    project,
  });

  const fileLines = fileContent.split("\n");
  let functionsIndexed = 0;
  let typesIndexed = 0;
  let edgesCreated = 0;
  const callEdges: SingleFileResult["callEdges"] = [];

  for (const sym of parsed.symbols) {
    if (isFunctionLike(sym.kind)) {
      const id = `fn:${absPath}:${sym.name}:${sym.lineStart}`;
      const body = fileLines.slice(sym.lineStart - 1, sym.lineEnd).join("\n");

      await mergeNode(conn, "Function", {
        id,
        name: sym.name,
        file: absPath,
        line: sym.lineStart,
        sig: (sym.signature ?? "").slice(0, 500),
        body: body.slice(0, 10000),
        docstring: (sym.docstring ?? "").slice(0, 2000),
        complexity: computeComplexity(body),
        project,
      });
      functionsIndexed++;

      await mergeEdge(conn, "File", fileId, "Function", id, "EXPORTS_FN");
      edgesCreated++;

      // Collect call edges for later resolution
      for (const edge of parsed.edges.filter((e) => e.callerName === sym.name)) {
        callEdges.push({ callerName: edge.callerName, calleeName: edge.calleeName, callerId: id });
      }
    } else {
      const id = `type:${absPath}:${sym.name}:${sym.lineStart}`;
      await mergeNode(conn, "Type", {
        id,
        name: sym.name,
        file: absPath,
        line: sym.lineStart,
        kind: sym.kind,
        project,
      });
      typesIndexed++;

      await mergeEdge(conn, "File", fileId, "Type", id, "EXPORTS_TYPE");
      edgesCreated++;
    }
  }

  // Tagged comments
  const comments = extractTaggedComments(absPath, fileContent);
  for (const c of comments) {
    await mergeNode(conn, "Comment", {
      id: c.id,
      text: c.text.slice(0, 2000),
      tag: c.tag,
      line: c.line,
      file: absPath,
      project,
    });

    const followingFn = parsed.symbols
      .filter((s) => isFunctionLike(s.kind) && s.lineStart >= c.line)
      .sort((a, b) => a.lineStart - b.lineStart)[0];
    if (followingFn) {
      const fnId = `fn:${absPath}:${followingFn.name}:${followingFn.lineStart}`;
      await mergeEdge(conn, "Comment", c.id, "Function", fnId, "ANNOTATES_FN");
      edgesCreated++;
    } else {
      const followingType = parsed.symbols
        .filter((s) => !isFunctionLike(s.kind) && s.lineStart >= c.line)
        .sort((a, b) => a.lineStart - b.lineStart)[0];
      if (followingType) {
        const typeId = `type:${absPath}:${followingType.name}:${followingType.lineStart}`;
        await mergeEdge(conn, "Comment", c.id, "Type", typeId, "ANNOTATES_TYPE");
        edgesCreated++;
      }
    }
  }

  return { functionsIndexed, typesIndexed, edgesCreated, imports: parsed.imports, callEdges };
}

export async function indexRepo(opts: IndexOptions): Promise<IndexResult> {
  const start = Date.now();

  const include = opts.include ?? ["**/*.ts", "**/*.tsx"];
  const exclude = opts.exclude ?? [
    "**/node_modules/**",
    "**/dist/**",
    "**/*.spec.ts",
    "**/*.spec.tsx",
    "**/*.d.ts",
    "**/*.test.ts",
    "**/*.test.tsx",
  ];

  const { db, conn } = await getOrCreateDb(opts.repoId);
  const adapter = new TypeScriptAdapter();

  try {
    await ensureSchemaWithConn(conn);

    const files = await fg(include, {
      cwd: opts.repoPath,
      ignore: exclude,
      absolute: true,
    });

    let functionsIndexed = 0;
    let typesIndexed = 0;
    let edgesCreated = 0;

    // Collect cross-file data for second/third passes
    const allFileResults = new Map<string, SingleFileResult>();

    // First pass: index all files via indexSingleFile
    for (const absPath of files) {
      try {
        const result = await indexSingleFile(conn, absPath, opts.repoPath, opts.project, adapter);
        allFileResults.set(absPath, result);
        functionsIndexed += result.functionsIndexed;
        typesIndexed += result.typesIndexed;
        edgesCreated += result.edgesCreated;
      } catch {
        continue;
      }
    }

    // Build name→id map for CALLS resolution (across all files)
    const fnIdByName = new Map<string, string>();
    for (const [, result] of allFileResults) {
      for (const e of result.callEdges) {
        fnIdByName.set(e.callerName, e.callerId);
      }
    }

    // Second pass: IMPORTS edges
    for (const [absPath, result] of allFileResults) {
      const fileId = `file:${absPath}`;
      const fileDir = dirname(absPath);

      for (const imp of result.imports) {
        if (!imp.targetPath.startsWith(".")) continue;

        const resolved = resolve(fileDir, imp.targetPath);
        const candidates = [
          resolved,
          `${resolved}.ts`,
          `${resolved}.tsx`,
          `${resolved}/index.ts`,
          `${resolved}/index.tsx`,
        ];

        for (const c of candidates) {
          if (allFileResults.has(c)) {
            await mergeEdge(conn, "File", fileId, "File", `file:${c}`, "IMPORTS");
            edgesCreated++;
            break;
          }
        }
      }
    }

    // Third pass: CALLS edges
    for (const [, result] of allFileResults) {
      for (const edge of result.callEdges) {
        const calleeId = fnIdByName.get(edge.calleeName);
        if (calleeId && calleeId !== edge.callerId) {
          await mergeEdge(conn, "Function", edge.callerId, "Function", calleeId, "CALLS", { weight: 1.0 });
          edgesCreated++;
        }
      }
    }

    // Persist index state for drift detection
    let lastCommit: string | undefined;
    try {
      lastCommit = execFileSync("git", ["rev-parse", "HEAD"], { cwd: opts.repoPath }).toString().trim();
    } catch { /* not a git repo */ }

    const dbDir = join(homedir(), ".sensei", "projects", opts.repoId);
    await writeFile(
      join(dbDir, "index-state.json"),
      JSON.stringify({ lastCommit, indexedAt: new Date().toISOString(), repoPath: opts.repoPath }, null, 2),
    );

    return {
      filesIndexed: files.length,
      functionsIndexed,
      typesIndexed,
      edgesCreated,
      durationMs: Date.now() - start,
    };
  } finally {
    await conn.close();
    await db.close();
  }
}
