import { watch, existsSync } from "node:fs";
import { readFile, writeFile, mkdir, stat } from "node:fs/promises";
import { join, relative, resolve, dirname } from "node:path";
import { homedir } from "node:os";
import { createHash } from "node:crypto";
import { type Connection } from "kuzu";
import fg from "fast-glob";
import { TypeScriptAdapter } from "@sensei/engine";
import { getOrCreateDb, deleteFileFromGraph, indexSingleFile } from "./indexer.js";
import { ensureSchemaWithConn } from "./schema.js";

// ─── Types ────────────────────────────────────────────────────────────────────

export interface WatcherOptions {
  repoPath: string;
  repoId: string;
  project: string;
  include?: string[];
  exclude?: string[];
  /** Debounce window for file changes (ms). Default 300. */
  debounceMs?: number;
  /** Callback invoked after each incremental update cycle. */
  onUpdate?: (result: IncrementalResult) => void;
}

export interface IncrementalResult {
  added: number;
  updated: number;
  removed: number;
  durationMs: number;
}

export interface WatcherHandle {
  /** Stop watching and close the DB connection. */
  stop(): Promise<void>;
  /** Force a full rescan (compares manifest vs disk). */
  rescan(): Promise<IncrementalResult>;
}

// ─── Manifest ─────────────────────────────────────────────────────────────────

interface ManifestEntry {
  mtime: number;
  hash: string;
}

type Manifest = Record<string, ManifestEntry>;

function manifestPath(repoId: string): string {
  return join(homedir(), ".sensei", "projects", repoId, "manifest.json");
}

async function loadManifest(repoId: string): Promise<Manifest> {
  try {
    const raw = await readFile(manifestPath(repoId), "utf-8");
    return JSON.parse(raw) as Manifest;
  } catch {
    return {};
  }
}

async function saveManifest(repoId: string, manifest: Manifest): Promise<void> {
  const p = manifestPath(repoId);
  await mkdir(dirname(p), { recursive: true });
  await writeFile(p, JSON.stringify(manifest, null, 2));
}

async function fileHash(absPath: string): Promise<string> {
  const content = await readFile(absPath);
  return createHash("sha256").update(content).digest("hex").slice(0, 16);
}

// ─── Core ─────────────────────────────────────────────────────────────────────

const DEFAULT_INCLUDE = ["**/*.ts", "**/*.tsx"];
const DEFAULT_EXCLUDE = [
  "**/node_modules/**",
  "**/dist/**",
  "**/*.spec.ts",
  "**/*.spec.tsx",
  "**/*.d.ts",
  "**/*.test.ts",
  "**/*.test.tsx",
];

async function doRescan(
  conn: Connection,
  adapter: TypeScriptAdapter,
  opts: WatcherOptions,
  manifest: Manifest
): Promise<{ result: IncrementalResult; updated: Manifest }> {
  const start = Date.now();
  const include = opts.include ?? DEFAULT_INCLUDE;
  const exclude = opts.exclude ?? DEFAULT_EXCLUDE;

  const files = await fg(include, {
    cwd: opts.repoPath,
    ignore: exclude,
    absolute: true,
  });

  const fileSet = new Set(files);
  let added = 0;
  let updated = 0;
  let removed = 0;

  // Detect removed files
  for (const knownPath of Object.keys(manifest)) {
    if (!fileSet.has(knownPath)) {
      await deleteFileFromGraph(conn, knownPath, opts.project);
      delete manifest[knownPath];
      removed++;
    }
  }

  // Detect new/changed files
  for (const absPath of files) {
    let s: { mtimeMs: number } | null = null;
    try {
      s = await stat(absPath);
    } catch {
      continue;
    }

    const mtime = Math.floor(s.mtimeMs);
    const existing = manifest[absPath];

    if (existing && existing.mtime === mtime) continue; // unchanged

    // Hash check for files where mtime is unreliable (e.g. git checkout)
    const hash = await fileHash(absPath);
    if (existing && existing.hash === hash) {
      manifest[absPath] = { mtime, hash };
      continue;
    }

    const isNew = !existing;

    try {
      if (!isNew) {
        await deleteFileFromGraph(conn, absPath, opts.project);
      }
      await indexSingleFile(conn, absPath, opts.repoPath, opts.project, adapter);
      manifest[absPath] = { mtime, hash };
      if (isNew) added++; else updated++;
    } catch {
      // Leave stale manifest entry so we retry next scan
    }
  }

  return {
    result: { added, updated, removed, durationMs: Date.now() - start },
    updated: manifest,
  };
}

// ─── Public API ───────────────────────────────────────────────────────────────

/**
 * Start watching a repo directory for changes.
 *
 * - On first call (empty manifest): performs a full initial index.
 * - On subsequent calls: only indexes new/changed files, removes deleted ones.
 * - Uses `fs.watch` with recursive option for low-latency change detection.
 * - Debounces rapid changes (e.g. bulk saves, git checkout) before indexing.
 *
 * Returns a handle to stop the watcher and optionally trigger a manual rescan.
 */
export async function watchRepo(opts: WatcherOptions): Promise<WatcherHandle> {
  const debounceMs = opts.debounceMs ?? 300;
  const { db, conn } = await getOrCreateDb(opts.repoId);
  await ensureSchemaWithConn(conn);

  const adapter = new TypeScriptAdapter();
  const manifest = await loadManifest(opts.repoId);

  let stopped = false;
  let debounceTimer: ReturnType<typeof setTimeout> | null = null;
  const pendingPaths = new Set<string>();
  let fsWatcher: ReturnType<typeof watch> | null = null;

  async function handleChange(absPath: string): Promise<void> {
    if (stopped) return;

    const include = opts.include ?? DEFAULT_INCLUDE;
    const exclude = opts.exclude ?? DEFAULT_EXCLUDE;

    // Filter: only handle files matching include/exclude patterns
    const relPath = relative(opts.repoPath, absPath);
    const matchesInclude = include.some((p) =>
      new RegExp(p.replace(/\*\*/g, ".*").replace(/\*/g, "[^/]*")).test(relPath)
    );
    if (!matchesInclude) return;
    const matchesExclude = exclude.some((p) =>
      new RegExp(p.replace(/\*\*/g, ".*").replace(/\*/g, "[^/]*")).test(relPath)
    );
    if (matchesExclude) return;

    if (!existsSync(absPath)) {
      // File deleted
      if (manifest[absPath]) {
        await deleteFileFromGraph(conn, absPath, opts.project);
        delete manifest[absPath];
        await saveManifest(opts.repoId, manifest);
        opts.onUpdate?.({ added: 0, updated: 0, removed: 1, durationMs: 0 });
      }
      return;
    }

    pendingPaths.add(absPath);

    if (debounceTimer) clearTimeout(debounceTimer);
    debounceTimer = setTimeout(async () => {
      if (stopped) return;
      const paths = [...pendingPaths];
      pendingPaths.clear();

      const start = Date.now();
      let added = 0;
      let updated = 0;

      for (const p of paths) {
        try {
          const s = await stat(p);
          const mtime = Math.floor(s.mtimeMs);
          const hash = await fileHash(p);
          const existing = manifest[p];

          if (existing?.hash === hash) {
            manifest[p] = { mtime, hash };
            continue;
          }

          const isNew = !existing;
          if (!isNew) await deleteFileFromGraph(conn, p, opts.project);
          await indexSingleFile(conn, p, opts.repoPath, opts.project, adapter);
          manifest[p] = { mtime, hash };
          if (isNew) added++; else updated++;
        } catch {
          // skip failed files
        }
      }

      await saveManifest(opts.repoId, manifest);
      if (added + updated > 0) {
        opts.onUpdate?.({ added, updated, removed: 0, durationMs: Date.now() - start });
      }
    }, debounceMs);
  }

  // Perform initial rescan to catch anything that changed since last run
  const { result: initialResult, updated: updatedManifest } = await doRescan(
    conn,
    adapter,
    opts,
    manifest
  );

  Object.assign(manifest, updatedManifest);
  await saveManifest(opts.repoId, manifest);

  if (initialResult.added + initialResult.updated + initialResult.removed > 0) {
    opts.onUpdate?.(initialResult);
  }

  // Start watching — Node 22+ supports recursive on macOS/Windows
  try {
    fsWatcher = watch(opts.repoPath, { recursive: true }, (_event, filename) => {
      if (filename && !stopped) {
        const absPath = join(opts.repoPath, filename);
        handleChange(absPath).catch(() => {});
      }
    });
  } catch {
    // fs.watch may fail on some systems; rescan-only mode is still useful
  }

  return {
    async stop(): Promise<void> {
      stopped = true;
      if (debounceTimer) clearTimeout(debounceTimer);
      fsWatcher?.close();
      await conn.close();
      await db.close();
    },

    async rescan(): Promise<IncrementalResult> {
      const { result, updated } = await doRescan(conn, adapter, opts, manifest);
      Object.assign(manifest, updated);
      await saveManifest(opts.repoId, manifest);
      opts.onUpdate?.(result);
      return result;
    },
  };
}
