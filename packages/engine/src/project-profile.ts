import { readFile } from "fs/promises";
import { join } from "path";
import { existsSync } from "fs";
import type { SupabaseClient } from "@supabase/supabase-js";
import type { ProjectProfile } from "@sensei/shared";

const EXT_TO_LANGUAGE: Record<string, string> = {
  ts: "typescript", tsx: "typescript",
  js: "javascript", jsx: "javascript",
  py: "python", go: "go", rs: "rust", rb: "ruby", java: "java",
};

export async function extractProjectProfile(
  db: SupabaseClient,
  repoId: string,
  repoPath: string,
): Promise<ProjectProfile> {
  // 1. Fetch repo name
  const { data: repoRow, error: repoError } = await db
    .from("repos")
    .select("name")
    .eq("id", repoId)
    .single();
  if (repoError || !repoRow) throw new Error(repoError?.message ?? "Repo not found");

  // 2. Fetch symbols for language detection + key symbol names
  const { data: symbolRows, error: symbolError } = await db
    .from("symbols")
    .select("name,file_path")
    .eq("repo_id", repoId)
    .order("name", { ascending: true })
    .limit(100);
  if (symbolError) throw new Error(symbolError.message);

  const symbols = (symbolRows ?? []) as Array<{ name: string; file_path: string }>;

  // Dominant language from file extension frequency
  const extCounts: Record<string, number> = {};
  for (const sym of symbols) {
    const ext = sym.file_path.split(".").at(-1) ?? "unknown";
    extCounts[ext] = (extCounts[ext] ?? 0) + 1;
  }
  const topExt = Object.entries(extCounts).sort((a, b) => b[1] - a[1])[0]?.[0] ?? "unknown";
  const dominantLanguage = EXT_TO_LANGUAGE[topExt] ?? topExt;

  const keySymbols = symbols.slice(0, 20).map(s => s.name);

  // 3. Read package.json
  const pkgPath = join(repoPath, "package.json");
  if (!existsSync(pkgPath)) throw new Error(`package.json not found at ${pkgPath}`);
  const pkg = JSON.parse(await readFile(pkgPath, "utf-8")) as {
    scripts?: Record<string, string>;
    dependencies?: Record<string, string>;
    devDependencies?: Record<string, string>;
    workspaces?: string[] | { packages: string[] };
  };

  const cliCommands = pkg.scripts ?? {};
  const allDeps = { ...(pkg.dependencies ?? {}), ...(pkg.devDependencies ?? {}) };

  // Framework detection
  let framework: string | null = null;
  if ("@sveltejs/kit" in allDeps) framework = "sveltekit";
  else if ("react" in allDeps) framework = "react";
  else if ("vue" in allDeps) framework = "vue";
  else if ("express" in allDeps) framework = "express";
  else if ("fastify" in allDeps) framework = "fastify";

  // Package names from workspaces
  let packageNames: string[] = [];
  if (pkg.workspaces) {
    const ws = Array.isArray(pkg.workspaces) ? pkg.workspaces : pkg.workspaces.packages;
    packageNames = ws.map(p => p.replace(/^packages\//, "").replace(/\/\*$/, ""));
  }

  // Test pattern
  const testPattern = "vitest" in allDeps ? "*.spec.ts" : "jest" in allDeps ? "*.test.ts" : "*.spec.ts";

  // 4. Read .sensei/config.yaml (best-effort)
  let senseiConfig = "";
  const configPath = join(repoPath, ".sensei", "config.yaml");
  if (existsSync(configPath)) {
    senseiConfig = await readFile(configPath, "utf-8");
  }

  return {
    repoName: repoRow.name as string,
    repoPath,
    dominantLanguage,
    framework,
    packageNames,
    keySymbols,
    testPattern,
    cliCommands,
    senseiConfig,
  };
}
