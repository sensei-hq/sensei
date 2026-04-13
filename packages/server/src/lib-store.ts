/**
 * Shared library documentation store.
 *
 * Stores lib docs centrally at ~/.sensei/libraries/{libName}/
 * instead of per-repo in activity.db. This means indexing rokkit docs
 * once makes them available to all repos that use rokkit.
 */
import { Database } from "bun:sqlite";
import { randomUUID } from "crypto";
import { join } from "path";
import { homedir } from "os";
import { mkdirSync, existsSync } from "fs";
import { readFile, writeFile, mkdir, readdir } from "fs/promises";

const LIBS_DIR = join(homedir(), ".sensei", "libraries");

interface LibMeta {
  name: string;
  sourceType: string;
  baseUrl?: string;
  usedBy: string[]; // repoIds
  indexedAt?: string;
}

// ─── Meta ────────────────────────────────────────────────────────────────────

function metaPath(libName: string): string {
  return join(LIBS_DIR, libName, "meta.json");
}

export async function getLibMeta(libName: string): Promise<LibMeta | null> {
  try {
    return JSON.parse(await readFile(metaPath(libName), "utf-8")) as LibMeta;
  } catch {
    return null;
  }
}

export async function writeLibMeta(libName: string, meta: LibMeta): Promise<void> {
  const dir = join(LIBS_DIR, libName);
  await mkdir(dir, { recursive: true });
  await writeFile(metaPath(libName), JSON.stringify(meta, null, 2));
}

export async function addLibUser(libName: string, repoId: string): Promise<void> {
  const meta = await getLibMeta(libName);
  if (!meta) return;
  if (!meta.usedBy.includes(repoId)) {
    meta.usedBy.push(repoId);
    await writeLibMeta(libName, meta);
  }
}

// ─── Docs DB ─────────────────────────────────────────────────────────────────

function getLibDb(libName: string): Database {
  const dir = join(LIBS_DIR, libName);
  if (!existsSync(dir)) mkdirSync(dir, { recursive: true });
  const db = new Database(join(dir, "docs.db"));
  db.exec(`
    CREATE TABLE IF NOT EXISTS lib_docs (
      id TEXT PRIMARY KEY,
      lib_name TEXT NOT NULL,
      title TEXT NOT NULL,
      url TEXT,
      local_path TEXT,
      summary TEXT NOT NULL DEFAULT '',
      content TEXT,
      source_type TEXT NOT NULL,
      component TEXT,
      indexed_at TEXT NOT NULL
    );
    CREATE INDEX IF NOT EXISTS lib_docs_lib ON lib_docs(lib_name);
  `);
  return db;
}

export function replaceLibDocs(
  libName: string,
  docs: Array<{
    title: string;
    url?: string;
    localPath?: string;
    summary: string;
    content?: string;
    sourceType: string;
    component?: string;
  }>,
): void {
  const db = getLibDb(libName);
  try {
    const deleteStmt = db.prepare(`DELETE FROM lib_docs WHERE lib_name = ?`);
    const insertStmt = db.prepare(
      `INSERT INTO lib_docs (id, lib_name, title, url, local_path, summary, content, source_type, component, indexed_at)
       VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?)`
    );
    const now = new Date().toISOString();
    const runAll = db.transaction(() => {
      deleteStmt.run(libName);
      for (const doc of docs) {
        insertStmt.run(
          randomUUID(),
          libName,
          doc.title,
          doc.url ?? null,
          doc.localPath ?? null,
          doc.summary,
          doc.content ?? null,
          doc.sourceType,
          doc.component ?? null,
          now,
        );
      }
    });
    runAll();
  } finally {
    db.close();
  }
}

export function getLibDocs(
  libName: string,
  opts?: { component?: string; query?: string; limit?: number },
): Array<{
  id: string;
  title: string;
  url: string | null;
  localPath: string | null;
  summary: string;
  content: string | null;
  sourceType: string;
  component: string | null;
}> {
  const dir = join(LIBS_DIR, libName);
  if (!existsSync(join(dir, "docs.db"))) return [];

  const db = getLibDb(libName);
  try {
    const limit = opts?.limit ?? 50;
    const conditions: string[] = ["lib_name = ?"];
    const params: (string | number)[] = [libName];

    if (opts?.component) {
      conditions.push("component = ?");
      params.push(opts.component);
    }
    if (opts?.query) {
      conditions.push("(title LIKE ? OR content LIKE ?)");
      params.push(`%${opts.query}%`, `%${opts.query}%`);
    }
    params.push(limit);

    const rows = db
      .prepare(
        `SELECT id, title, url, local_path, summary, content, source_type, component
         FROM lib_docs
         WHERE ${conditions.join(" AND ")}
         ORDER BY title
         LIMIT ?`
      )
      .all(...params) as Record<string, unknown>[];

    return rows.map((r) => ({
      id: r["id"] as string,
      title: r["title"] as string,
      url: r["url"] as string | null,
      localPath: r["local_path"] as string | null,
      summary: r["summary"] as string,
      content: r["content"] as string | null,
      sourceType: r["source_type"] as string,
      component: r["component"] as string | null,
    }));
  } finally {
    db.close();
  }
}

/** List all libraries that have indexed docs. */
export async function listLibraries(): Promise<LibMeta[]> {
  if (!existsSync(LIBS_DIR)) return [];
  const dirs = await readdir(LIBS_DIR);
  const results: LibMeta[] = [];
  for (const d of dirs) {
    const meta = await getLibMeta(d);
    if (meta) results.push(meta);
  }
  return results;
}
