/**
 * Shared types, constants, and helpers used by both indexer.ts and watcher.ts.
 * Extracted to eliminate duplication between the full-index and watch paths.
 */
import { readFile, writeFile, mkdir } from "node:fs/promises";
import { join, dirname } from "node:path";
import { homedir } from "node:os";
import { createHash } from "node:crypto";
import {
  TypeScriptAdapter, SvelteAdapter,
  RustAdapter, PythonAdapter, SqlAdapter,
  JavaAdapter, KotlinAdapter, SwiftAdapter,
} from "@sensei/engine";

// ─── Supported extensions ────────────────────────────────────────────────────

export const SUPPORTED_EXTENSIONS = [
  ".ts", ".tsx", ".js", ".jsx", ".mjs", ".cjs", ".svelte",
  ".rs", ".py", ".sql",
  ".java", ".kt", ".kts",
  ".swift",
];

export const DEFAULT_CODE_INCLUDE = SUPPORTED_EXTENSIONS.map(ext => `**/*${ext}`);
export const DEFAULT_DOC_INCLUDE = ["**/*.md", "**/*.mdx"];
export const DEFAULT_INCLUDE = [...DEFAULT_CODE_INCLUDE, ...DEFAULT_DOC_INCLUDE];
export const DEFAULT_EXCLUDE = [
  "**/node_modules/**",
  "**/dist/**",
  "**/build/**",
  "**/target/**",
  "**/.next/**",
  "**/.svelte-kit/**",
  "**/__pycache__/**",
  "**/.venv/**",
  "**/venv/**",
  "**/*.spec.ts", "**/*.spec.tsx", "**/*.spec.js",
  "**/*.test.ts", "**/*.test.tsx", "**/*.test.js",
  "**/*_test.py", "**/*_test.go", "**/*_test.rs",
  "**/*.d.ts",
];

export function isDocFile(absPath: string): boolean {
  return absPath.endsWith(".md") || absPath.endsWith(".mdx");
}

// ─── Manifest ────────────────────────────────────────────────────────────────

export interface ManifestEntry {
  mtime: number;
  hash: string;
}

export type Manifest = Record<string, ManifestEntry>;

export function manifestPath(repoId: string): string {
  return join(homedir(), ".sensei", "projects", repoId, "manifest.json");
}

export async function loadManifest(repoId: string): Promise<Manifest> {
  try {
    const raw = await readFile(manifestPath(repoId), "utf-8");
    return JSON.parse(raw) as Manifest;
  } catch {
    return {};
  }
}

export async function saveManifest(repoId: string, manifest: Manifest): Promise<void> {
  const p = manifestPath(repoId);
  await mkdir(dirname(p), { recursive: true });
  await writeFile(p, JSON.stringify(manifest, null, 2));
}

export async function fileHash(absPath: string): Promise<string> {
  const content = await readFile(absPath);
  return createHash("sha256").update(content).digest("hex").slice(0, 16);
}

// ─── Language adapter map ────────────────────────────────────────────────────

export type LanguageAdapter = { parse: TypeScriptAdapter["parse"] };
export type AdapterMap = Map<string, LanguageAdapter>;

export function buildAdapterMap(): AdapterMap {
  const ts = new TypeScriptAdapter();
  const svelte = new SvelteAdapter();
  const rust = new RustAdapter();
  const python = new PythonAdapter();
  const sql = new SqlAdapter();
  const java = new JavaAdapter();
  const kotlin = new KotlinAdapter();
  const swift = new SwiftAdapter();

  const map: AdapterMap = new Map();
  for (const ext of [".ts", ".tsx", ".js", ".jsx", ".mjs", ".cjs"]) map.set(ext, ts);
  map.set(".svelte", svelte);
  map.set(".rs", rust);
  map.set(".py", python);
  map.set(".sql", sql);
  map.set(".java", java);
  for (const ext of [".kt", ".kts"]) map.set(ext, kotlin);
  map.set(".swift", swift);
  return map;
}

// ─── Cypher escaping ─────────────────────────────────────────────────────────

export function escapeCypherStr(s: string): string {
  return s
    .replace(/\\/g, "\\\\")
    .replace(/'/g, "\\'")
    .replace(/\n/g, "\\n")
    .replace(/\r/g, "\\r")
    .replace(/\t/g, "\\t");
}
