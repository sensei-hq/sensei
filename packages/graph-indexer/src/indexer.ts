import {
  type Database,
  type Connection,
  type QueryResult,
  type KuzuValue,
} from "kuzu";
import fg from "fast-glob";
import { readFile, mkdir, writeFile, unlink } from "node:fs/promises";
import { join, resolve, dirname, relative, extname } from "node:path";
import { homedir } from "node:os";
import { existsSync } from "node:fs";
import { stat } from "node:fs/promises";
import { execFileSync } from "node:child_process";
import { extractTaggedComments } from "./parser.js";
import { ensureSchemaWithConn } from "./schema.js";
import { indexDocFile, writeTraceability } from "./doc-indexer.js";
import {
  SUPPORTED_EXTENSIONS,
  DEFAULT_CODE_INCLUDE,
  DEFAULT_EXCLUDE,
  type Manifest,
  type LanguageAdapter,
  loadManifest,
  saveManifest,
  fileHash,
  buildAdapterMap,
  escapeCypherStr,
} from "./shared.js";
import { detectWorkspacePackages, collectExternalLibs } from "./lib-detector.js";

const FUNCTION_KINDS = new Set(["function", "method", "component", "hook"]);

function isFunctionLike(kind: string): boolean {
  return FUNCTION_KINDS.has(kind);
}

// Re-export for downstream consumers
export { SUPPORTED_EXTENSIONS } from "./shared.js";

export interface IndexOptions {
  repoPath: string;
  repoId: string;
  project: string; // display name
  include?: string[]; // glob patterns — defaults to all SUPPORTED_EXTENSIONS
  exclude?: string[]; // default excludes node_modules, dist, test/spec files, type declarations
}

export interface IndexResult {
  filesIndexed: number;
  filesSkipped: number;
  functionsIndexed: number;
  typesIndexed: number;
  edgesCreated: number;
  durationMs: number;
  docsIndexed: number;
  /** External libraries detected from imports, grouped by org scope. */
  libs: string[];
}


export interface IndexProgress {
  repoId: string;
  currentFile: string;   // relative path of the file being processed right now
  filesProcessed: number;
  filesTotal: number;
  filesUnchanged: number;
  startedAt: string;
}

async function writeProgress(repoId: string, progress: IndexProgress): Promise<void> {
  const dbDir = join(homedir(), ".sensei", "projects", repoId);
  await writeFile(join(dbDir, "progress.json"), JSON.stringify(progress)).catch(() => {});
}

export async function clearProgress(repoId: string): Promise<void> {
  const p = join(homedir(), ".sensei", "projects", repoId, "progress.json");
  await unlink(p).catch(() => {});
}

export function progressPath(repoId: string): string {
  return join(homedir(), ".sensei", "projects", repoId, "progress.json");
}

// ─────────────────────────────────────────────────────────────────────────────

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
  /** All function-like symbols: name → graph node ID */
  fnIds: Map<string, string>;
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
  adapter: LanguageAdapter
): Promise<SingleFileResult> {
  const relPath = relative(repoPath, absPath);
  const parsed = await adapter.parse({ path: relPath, absPath, mtime: 0, hash: "", size: 0 });
  const fileContent = await readFile(absPath, "utf-8");

  const fileId = `file:${absPath}`;
  const moduleName = relPath.replace(/\.[^./\\]+$/, "").replace(/\\/g, "/");

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
  const fnIds = new Map<string, string>();

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
      fnIds.set(sym.name, id);

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

  return { functionsIndexed, typesIndexed, edgesCreated, imports: parsed.imports, callEdges, fnIds };
}

export async function indexRepo(opts: IndexOptions): Promise<IndexResult> {
  const start = Date.now();

  const include = opts.include ?? DEFAULT_CODE_INCLUDE;
  const exclude = opts.exclude ?? DEFAULT_EXCLUDE;

  const { db, conn } = await getOrCreateDb(opts.repoId);
  const adapterMap = buildAdapterMap();

  try {
    await ensureSchemaWithConn(conn);

    const files = await fg(include, {
      cwd: opts.repoPath,
      ignore: exclude,
      absolute: true,
    });

    // Load manifest to enable incremental indexing and per-file resume.
    // Each file is written to Kuzu immediately, and the manifest is updated
    // right after, so an interrupted run resumes from the last checkpoint.
    const manifest = await loadManifest(opts.repoId);
    const fileSet = new Set(files);

    // Remove graph nodes for files that no longer exist
    for (const knownPath of Object.keys(manifest)) {
      if (!fileSet.has(knownPath)) {
        await deleteFileFromGraph(conn, knownPath, opts.project).catch(() => {});
        delete manifest[knownPath];
      }
    }

    let functionsIndexed = 0;
    let typesIndexed = 0;
    let edgesCreated = 0;
    let filesSkipped = 0;
    let filesUnchanged = 0;
    let filesProcessed = 0;
    const startedAt = new Date().toISOString();
    const fileErrors: { path: string; error: string }[] = [];

    // Write an initial progress entry immediately so the UI shows the repo as active
    // even during the fast unchanged-file scan phase.
    await writeProgress(opts.repoId, {
      repoId: opts.repoId,
      currentFile: '(scanning…)',
      filesProcessed: 0,
      filesTotal: files.length,
      filesUnchanged: 0,
      startedAt,
    });

    // Collect cross-file data for second/third passes (only changed/new files)
    const allFileResults = new Map<string, SingleFileResult>();

    // First pass: incremental index — skip files that haven't changed since last run
    for (const absPath of files) {
      const ext = extname(absPath).toLowerCase();
      const adapter = adapterMap.get(ext);
      if (!adapter) {
        filesSkipped++;
        fileErrors.push({ path: absPath, error: `No adapter for extension: ${ext}` });
        continue;
      }

      // Check manifest: skip if mtime unchanged, verify with hash for git-checkout safety
      try {
        const s = await stat(absPath);
        const mtime = Math.floor(s.mtimeMs);
        const existing = manifest[absPath];
        if (existing) {
          if (existing.mtime === mtime) {
            filesUnchanged++;
            continue; // fast path: mtime exact match
          }
          const hash = await fileHash(absPath);
          if (existing.hash === hash) {
            manifest[absPath] = { mtime, hash }; // update mtime only
            filesUnchanged++;
            continue;
          }
          // File changed — delete old graph nodes before re-indexing
          await deleteFileFromGraph(conn, absPath, opts.project).catch(() => {});
        }
      } catch {
        // Can't stat — try to index anyway
      }

      // Emit live progress before processing so the UI shows the file being worked on
      await writeProgress(opts.repoId, {
        repoId: opts.repoId,
        currentFile: relative(opts.repoPath, absPath),
        filesProcessed,
        filesTotal: files.length,
        filesUnchanged,
        startedAt,
      });

      try {
        const result = await indexSingleFile(conn, absPath, opts.repoPath, opts.project, adapter);
        allFileResults.set(absPath, result);
        functionsIndexed += result.functionsIndexed;
        typesIndexed += result.typesIndexed;
        edgesCreated += result.edgesCreated;
        filesProcessed++;
        // Checkpoint: write manifest entry immediately so a restart skips this file
        const hash = await fileHash(absPath).catch(() => "");
        const mtime = await stat(absPath).then(s => Math.floor(s.mtimeMs)).catch(() => 0);
        manifest[absPath] = { mtime, hash };
        await saveManifest(opts.repoId, manifest).catch(() => {});
      } catch (err) {
        filesSkipped++;
        fileErrors.push({ path: absPath, error: err instanceof Error ? err.message : String(err) });
        // Continue — one bad file should never abort the whole repo
      }
    }

    // Build name→id map for CALLS resolution (across all files).
    // Uses fnIds from each file result so ALL function nodes are included,
    // not just those that happen to be callers.
    const fnIdByName = new Map<string, string>();
    for (const [, result] of allFileResults) {
      for (const [name, id] of result.fnIds) {
        fnIdByName.set(name, id);
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
          `${resolved}.ts`, `${resolved}.tsx`,
          `${resolved}.js`, `${resolved}.jsx`,
          `${resolved}.svelte`,
          `${resolved}/index.ts`, `${resolved}/index.tsx`,
          `${resolved}/index.js`, `${resolved}/index.jsx`,
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

    // Pass 4: index doc files (.md / .mdx) — builds Doc nodes + COVERS/MENTIONS_FN/SUPERSEDES edges
    const docFiles = await fg(["**/*.md", "**/*.mdx"], {
      cwd: opts.repoPath,
      ignore: ["**/node_modules/**", "**/dist/**"],
      absolute: true,
    });

    const fileAbsPaths = new Set(files);
    let docsIndexed = 0;

    for (const docPath of docFiles) {
      try {
        // Manifest check: skip unchanged doc files
        const docStat = await stat(docPath);
        const docMtime = Math.floor(docStat.mtimeMs);
        const existingDoc = manifest[docPath];
        if (existingDoc && existingDoc.mtime === docMtime) {
          const docHash = await fileHash(docPath);
          if (existingDoc.hash === docHash) {
            manifest[docPath] = { mtime: docMtime, hash: docHash };
            continue;
          }
        }

        const docHash = existingDoc?.mtime !== docMtime ? await fileHash(docPath) : existingDoc.hash;
        const result = await indexDocFile(conn, docPath, opts.repoPath, opts.project, fnIdByName, fileAbsPaths);
        edgesCreated += result.edgesCreated;
        docsIndexed++;
        manifest[docPath] = { mtime: docMtime, hash: docHash };
      } catch {
        continue;
      }
    }

    // Write .sensei/traceability.json for checkDrift() compatibility
    await writeTraceability(conn, opts.project, opts.repoPath);

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

    // Write per-file errors so the server can surface them to the UI
    if (fileErrors.length > 0) {
      await writeFile(
        join(dbDir, "index-errors.json"),
        JSON.stringify(fileErrors, null, 2),
      ).catch(() => { /* non-fatal */ });
    }

    // Final manifest save (captures doc files and any mtime-only updates)
    await saveManifest(opts.repoId, manifest).catch(() => {});

    // Clear progress file — indexing is done
    await clearProgress(opts.repoId);

    // Detect external library usage from imports
    const allImports = [...allFileResults.values()].flatMap(r => r.imports);
    const workspacePackages = await detectWorkspacePackages(opts.repoPath);
    const externalLibs = collectExternalLibs(allImports, workspacePackages);
    const libs = [...externalLibs.keys()].sort();

    return {
      filesIndexed: files.length - filesSkipped - filesUnchanged,
      filesSkipped,
      functionsIndexed,
      typesIndexed,
      edgesCreated,
      durationMs: Date.now() - start,
      docsIndexed,
      libs,
    };
  } finally {
    await conn.close();
    await db.close();
  }
}
