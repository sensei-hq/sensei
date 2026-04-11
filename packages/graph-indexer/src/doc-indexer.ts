/**
 * Doc indexer — indexes .md/.mdx files into the Kuzu graph.
 *
 * Creates:
 *   (d:Doc)-[:COVERS]->(f:File)          doc references a code file
 *   (d:Doc)-[:MENTIONS_FN]->(fn:Function) doc mentions a known function
 *   (d:Doc)-[:SUPERSEDES]->(d2:Doc)       from frontmatter supersedes: list
 *
 * Also writes <repoPath>/.sensei/traceability.json for checkDrift() compatibility.
 */

import { readFile, mkdir, writeFile } from "node:fs/promises";
import { relative, resolve, dirname, join } from "node:path";
import { type Connection, type QueryResult } from "kuzu";
import { escapeCypherStr } from "./shared.js";

// ─── Frontmatter parsing ──────────────────────────────────────────────────────

const FRONTMATTER_RE = /^---\r?\n([\s\S]*?)\r?\n---/;

/**
 * Extract a YAML block list from raw frontmatter text.
 * Handles:
 *   key:
 *     - value1
 *     - value2
 */
function parseBlockList(raw: string, key: string): string[] {
  const re = new RegExp(`(?:^|\\n)${key}:\\s*\\n((?:\\s+-[^\\n]+\\n?)+)`);
  const m = re.exec(raw);
  if (!m) return [];
  return m[1]
    .split("\n")
    .map((l) => l.replace(/^\s+-\s*/, "").trim().replace(/^['"]|['"]$/g, ""))
    .filter(Boolean);
}

/**
 * Extract a YAML inline list from raw frontmatter text.
 * Handles:  key: [value1, value2]
 */
function parseInlineList(raw: string, key: string): string[] {
  const re = new RegExp(`(?:^|\\n)${key}:\\s*\\[([^\\]]*)\\]`);
  const m = re.exec(raw);
  if (!m) return [];
  return m[1]
    .split(",")
    .map((s) => s.trim().replace(/^['"]|['"]$/g, ""))
    .filter(Boolean);
}

function parseFrontmatter(content: string): { covers: string[]; supersedes: string[] } {
  const m = FRONTMATTER_RE.exec(content);
  if (!m) return { covers: [], supersedes: [] };
  const raw = m[1];
  return {
    covers:    parseBlockList(raw, "covers").length > 0 ? parseBlockList(raw, "covers") : parseInlineList(raw, "covers"),
    supersedes: parseBlockList(raw, "supersedes").length > 0 ? parseBlockList(raw, "supersedes") : parseInlineList(raw, "supersedes"),
  };
}

function extractTitle(content: string): string {
  const body = content.replace(FRONTMATTER_RE, "");
  const m = /^#\s+(.+)/m.exec(body);
  return m ? m[1].trim() : "";
}

// ─── Content reference extraction ────────────────────────────────────────────

const CODE_EXTS = "ts|tsx|js|jsx|py|go|rs";

// `path/to/file.ts`  or  `./relative.ts`
const BACKTICK_PATH_RE = new RegExp(
  "`([./][\\w./-]+\\.(?:" + CODE_EXTS + "))`",
  "g"
);
// [text](path/to/file.ts)
const MD_LINK_CODE_RE = new RegExp(
  "\\]\\(([./][\\w./-]+\\.(?:" + CODE_EXTS + "))\\)",
  "g"
);
// bare repo-root paths like packages/foo/src/bar.ts
const BARE_PATH_RE = new RegExp(
  "\\b((?:packages|apps|src|lib|tools)/[\\w./-]+\\.(?:" + CODE_EXTS + "))\\b",
  "g"
);

function extractFileRefs(content: string, docAbsPath: string, repoPath: string): string[] {
  const body = content.replace(FRONTMATTER_RE, "");
  const raw = new Set<string>();

  for (const re of [BACKTICK_PATH_RE, MD_LINK_CODE_RE]) {
    re.lastIndex = 0;
    let m: RegExpExecArray | null;
    while ((m = re.exec(body)) !== null) raw.add(m[1]);
  }

  BARE_PATH_RE.lastIndex = 0;
  let m: RegExpExecArray | null;
  while ((m = BARE_PATH_RE.exec(body)) !== null) raw.add(m[1]);

  const docDir = dirname(docAbsPath);
  const resolved = new Set<string>();

  for (const ref of raw) {
    if (ref.startsWith(".")) {
      resolved.add(resolve(docDir, ref));
    } else {
      resolved.add(resolve(repoPath, ref));
    }
  }

  return [...resolved];
}

// ─── Function mention extraction ─────────────────────────────────────────────

const BACKTICK_IDENT_RE = /`([a-zA-Z_$][\w$]*)`/g;

function extractFnMentions(content: string): string[] {
  const body = content.replace(FRONTMATTER_RE, "");
  const names = new Set<string>();
  BACKTICK_IDENT_RE.lastIndex = 0;
  let m: RegExpExecArray | null;
  while ((m = BACKTICK_IDENT_RE.exec(body)) !== null) {
    // Accept plain identifiers only — no paths or dotted chains
    if (!m[1].includes("/") && !m[1].includes(".")) {
      names.add(m[1]);
    }
  }
  return [...names];
}

// ─── Kuzu helpers ─────────────────────────────────────────────────────────────

async function runQuery(conn: Connection, statement: string): Promise<void> {
  const r = (await conn.query(statement)) as QueryResult | QueryResult[];
  if (Array.isArray(r)) { for (const x of r) x.close(); } else { r.close(); }
}

async function mergeDocNode(
  conn: Connection,
  id: string,
  path: string,
  title: string,
  project: string
): Promise<void> {
  await runQuery(
    conn,
    `MERGE (d:Doc {id: '${escapeCypherStr(id)}'})
     SET d.path = '${escapeCypherStr(path)}',
         d.title = '${escapeCypherStr(title)}',
         d.project = '${escapeCypherStr(project)}'`
  );
}

async function mergeDocEdge(
  conn: Connection,
  fromId: string,
  toLabel: string,
  toId: string,
  rel: string
): Promise<void> {
  try {
    await runQuery(
      conn,
      `MATCH (a:Doc {id: '${escapeCypherStr(fromId)}'})
       MATCH (b:${toLabel} {id: '${escapeCypherStr(toId)}'})
       MERGE (a)-[:${rel}]->(b)`
    );
  } catch {
    // Target node may not exist yet — skip silently
  }
}

// ─── Public helpers ───────────────────────────────────────────────────────────

/** Build a name→id map from all Function nodes in the graph (for MENTIONS_FN edges). */
export async function loadFnIdMap(
  conn: Connection,
  project: string
): Promise<Map<string, string>> {
  const map = new Map<string, string>();
  try {
    const r = (await conn.query(
      `MATCH (f:Function {project: '${escapeCypherStr(project)}'}) RETURN f.name AS name, f.id AS id`
    )) as QueryResult | QueryResult[];
    const qr = Array.isArray(r) ? r[0] : r;
    const rows = (await qr.getAll()) as Record<string, unknown>[];
    if (Array.isArray(r)) { for (const x of r) x.close(); } else { r.close(); }
    for (const row of rows) {
      map.set(String(row["name"]), String(row["id"]));
    }
  } catch {
    // graph may be empty
  }
  return map;
}

/** Build the set of absolute file paths from all File nodes in the graph. */
export async function loadFileAbsPaths(
  conn: Connection,
  project: string
): Promise<Set<string>> {
  const set = new Set<string>();
  try {
    const r = (await conn.query(
      `MATCH (f:File {project: '${escapeCypherStr(project)}'}) RETURN f.path AS path`
    )) as QueryResult | QueryResult[];
    const qr = Array.isArray(r) ? r[0] : r;
    const rows = (await qr.getAll()) as Record<string, unknown>[];
    if (Array.isArray(r)) { for (const x of r) x.close(); } else { r.close(); }
    for (const row of rows) set.add(String(row["path"]));
  } catch {}
  return set;
}

// ─── Core indexer ─────────────────────────────────────────────────────────────

/**
 * Index a single .md/.mdx file into the Kuzu graph.
 *
 * @param fnIdByName   Map of function name → Kuzu id (from pass-1 results).
 *                     Pass an empty map to skip MENTIONS_FN edges.
 * @param fileAbsPaths Set of absolute paths of indexed File nodes.
 *                     Used to validate heuristic file refs found in content.
 */
export async function indexDocFile(
  conn: Connection,
  absPath: string,
  repoPath: string,
  project: string,
  fnIdByName: Map<string, string>,
  fileAbsPaths: Set<string>
): Promise<{ edgesCreated: number }> {
  const content = await readFile(absPath, "utf-8");
  const { covers: explicitCovers, supersedes } = parseFrontmatter(content);
  const title = extractTitle(content);
  const docId = `doc:${absPath}`;

  await mergeDocNode(conn, docId, absPath, title, project);

  let edgesCreated = 0;

  // COVERS — explicit frontmatter covers: list
  const explicitAbsPaths = new Set<string>();
  for (const ref of explicitCovers) {
    const absRef = ref.startsWith(".") ? resolve(dirname(absPath), ref) : resolve(repoPath, ref);
    explicitAbsPaths.add(absRef);
    const fileId = `file:${absRef}`;
    await mergeDocEdge(conn, docId, "File", fileId, "COVERS");
    edgesCreated++;
  }

  // COVERS — heuristic: file paths extracted from doc content
  for (const absRef of extractFileRefs(content, absPath, repoPath)) {
    if (fileAbsPaths.has(absRef) && !explicitAbsPaths.has(absRef)) {
      const fileId = `file:${absRef}`;
      await mergeDocEdge(conn, docId, "File", fileId, "COVERS");
      edgesCreated++;
    }
  }

  // MENTIONS_FN — backtick identifiers matching known function names
  if (fnIdByName.size > 0) {
    for (const name of extractFnMentions(content)) {
      const fnId = fnIdByName.get(name);
      if (fnId) {
        await mergeDocEdge(conn, docId, "Function", fnId, "MENTIONS_FN");
        edgesCreated++;
      }
    }
  }

  // SUPERSEDES — from frontmatter
  for (const ref of supersedes) {
    const absRef = ref.startsWith(".") ? resolve(dirname(absPath), ref) : resolve(repoPath, ref);
    const targetId = `doc:${absRef}`;
    await mergeDocEdge(conn, docId, "Doc", targetId, "SUPERSEDES");
    edgesCreated++;
  }

  return { edgesCreated };
}

/** Delete a Doc node and all its edges. Called before re-indexing a changed doc. */
export async function deleteDocFromGraph(
  conn: Connection,
  absPath: string
): Promise<void> {
  try {
    await runQuery(
      conn,
      `MATCH (d:Doc {id: '${escapeCypherStr(`doc:${absPath}`)}'}) DETACH DELETE d`
    );
  } catch {}
}

// ─── Traceability output ──────────────────────────────────────────────────────

/**
 * Write .sensei/traceability.json from the COVERS edges in the graph.
 * Format: { "relative/doc/path.md": ["relative/code/path.ts", ...] }
 * Compatible with checkDrift() in @sensei/tools.
 */
export async function writeTraceability(
  conn: Connection,
  project: string,
  repoPath: string
): Promise<void> {
  const traceability: Record<string, string[]> = {};

  try {
    const r = (await conn.query(
      `MATCH (d:Doc {project: '${escapeCypherStr(project)}'})-[:COVERS]->(f:File {project: '${escapeCypherStr(project)}'})
       RETURN d.path AS docPath, f.path AS filePath`
    )) as QueryResult | QueryResult[];
    const qr = Array.isArray(r) ? r[0] : r;
    const rows = (await qr.getAll()) as Record<string, unknown>[];
    if (Array.isArray(r)) { for (const x of r) x.close(); } else { r.close(); }

    for (const row of rows) {
      const docRel = relative(repoPath, String(row["docPath"]));
      const fileRel = relative(repoPath, String(row["filePath"]));
      if (!traceability[docRel]) traceability[docRel] = [];
      traceability[docRel].push(fileRel);
    }
  } catch {
    return;
  }

  if (Object.keys(traceability).length === 0) return;

  const outPath = join(repoPath, ".sensei", "traceability.json");
  await mkdir(dirname(outPath), { recursive: true });
  await writeFile(outPath, JSON.stringify(traceability, null, 2));
}
