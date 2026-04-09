import { readFile } from "fs/promises";
import { join } from "path";
import { existsSync } from "fs";
import type { ProjectProfile } from "@sensei/shared";

const EXT_TO_LANGUAGE: Record<string, string> = {
  ts: "typescript", tsx: "typescript",
  js: "javascript", jsx: "javascript",
  py: "python", go: "go", rs: "rust", rb: "ruby", java: "java",
};

function detectLanguageFromFs(repoPath: string): string {
  if (existsSync(join(repoPath, "pyproject.toml")) || existsSync(join(repoPath, "requirements.txt"))) return "python";
  if (existsSync(join(repoPath, "go.mod"))) return "go";
  if (existsSync(join(repoPath, "Cargo.toml"))) return "rust";
  return "typescript";
}

export async function extractProjectProfile(
  repoId: string,
  repoPath: string,
): Promise<ProjectProfile> {
  // 1. Read package.json for repo name, deps, and scripts
  const pkgPath = join(repoPath, "package.json");
  if (!existsSync(pkgPath)) throw new Error(`package.json not found at ${pkgPath}`);
  const pkg = JSON.parse(await readFile(pkgPath, "utf-8")) as {
    name?: string;
    scripts?: Record<string, string>;
    dependencies?: Record<string, string>;
    devDependencies?: Record<string, string>;
    workspaces?: string[] | { packages: string[] };
  };

  const repoName = pkg.name ?? repoPath.split("/").at(-1) ?? repoId;
  const cliCommands = pkg.scripts ?? {};
  const allDeps = { ...(pkg.dependencies ?? {}), ...(pkg.devDependencies ?? {}) };

  // 2. Language detection — filesystem first, then package.json deps
  let dominantLanguage = detectLanguageFromFs(repoPath);
  if (dominantLanguage === "typescript") {
    // Confirm via deps (handles JS-only projects)
    if (!("typescript" in allDeps) && !("@types/node" in allDeps) && ("python" in allDeps)) {
      dominantLanguage = "python";
    }
  }

  // 3. Framework detection
  let framework: string | null = null;
  if ("@sveltejs/kit" in allDeps) framework = "sveltekit";
  else if ("react" in allDeps) framework = "react";
  else if ("vue" in allDeps) framework = "vue";
  else if ("express" in allDeps) framework = "express";
  else if ("fastify" in allDeps) framework = "fastify";

  // 4. Package names from workspaces
  let packageNames: string[] = [];
  if (pkg.workspaces) {
    const ws = Array.isArray(pkg.workspaces) ? pkg.workspaces : pkg.workspaces.packages;
    packageNames = ws.map(p => p.replace(/^packages\//, "").replace(/\/\*$/, ""));
  }

  const testPattern = "vitest" in allDeps ? "*.spec.ts" : "jest" in allDeps ? "*.test.ts" : "*.spec.ts";

  // 5. Read .sensei/config.yaml (best-effort)
  let senseiConfig = "";
  const configPath = join(repoPath, ".sensei", "config.yaml");
  if (existsSync(configPath)) {
    senseiConfig = await readFile(configPath, "utf-8");
  }

  return {
    repoName,
    repoPath,
    dominantLanguage,
    framework,
    packageNames,
    keySymbols: [], // callers can supplement via graph query if available
    testPattern,
    cliCommands,
    senseiConfig,
  };
}
