/**
 * Central project registry — maps repo paths to repoIds.
 *
 * Source of truth: ~/.sensei/projects.json (managed by the sensei server).
 * Used by: MCP entry, CLI tools, resolveProject().
 *
 * This replaces the per-repo .sensei/config.yaml lookup for repo_id.
 * .sensei/config.yaml in the repo is still used for custom_libs config.
 */
import { readFile } from "fs/promises";
import { join, resolve } from "path";
import { homedir } from "os";

const PROJECTS_FILE = join(homedir(), ".sensei", "projects.json");

interface RegistryEntry {
  repoId: string;
  name: string;
  path: string;
  indexedAt?: string;
}

let _cache: RegistryEntry[] | null = null;
let _cacheTime = 0;
const CACHE_TTL = 5_000; // 5 seconds

/** Read the central project registry. Cached for 5s. */
export async function readRegistry(): Promise<RegistryEntry[]> {
  const now = Date.now();
  if (_cache && now - _cacheTime < CACHE_TTL) return _cache;
  try {
    _cache = JSON.parse(await readFile(PROJECTS_FILE, "utf-8")) as RegistryEntry[];
    _cacheTime = now;
    return _cache;
  } catch {
    return [];
  }
}

/** Invalidate the cache (call after writing projects.json). */
export function invalidateRegistryCache(): void {
  _cache = null;
}

/**
 * Look up the repoId for a given repo path.
 * Matches by comparing resolved absolute paths.
 * Returns undefined if the repo is not registered.
 */
export async function lookupRepoId(repoPath: string): Promise<string | undefined> {
  const resolved = resolve(repoPath);
  const entries = await readRegistry();

  // Exact match first
  const exact = entries.find(e => resolve(e.path) === resolved);
  if (exact) return exact.repoId;

  // Check if repoPath is a subdirectory of a registered repo (monorepo case)
  for (const e of entries) {
    if (resolved.startsWith(resolve(e.path) + "/")) return e.repoId;
  }

  return undefined;
}

/**
 * Look up the repo path for a given repoId.
 */
export async function lookupRepoPath(repoId: string): Promise<string | undefined> {
  const entries = await readRegistry();
  return entries.find(e => e.repoId === repoId)?.path;
}
