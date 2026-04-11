import { writeFile, mkdir, readFile } from "fs/promises";
import { join } from "path";
import { existsSync } from "fs";
import { SkillGenerator, SkillValidator, ClaudeAdapter } from "@sensei/engine";
import { ClaudeBackend } from "../model/claude-backend.js";
import type { AgentSkillsManifest, ProjectProfile } from "@sensei/shared";
import { getOrCreateDb, searchSymbols } from "@sensei/graph-indexer";
import { resolveProject } from "./resolve-project.js";

const EXT_TO_LANGUAGE: Record<string, string> = {
  ts: "typescript", tsx: "typescript",
  js: "javascript", jsx: "javascript",
  py: "python", go: "go", rs: "rust", rb: "ruby", java: "java",
};

async function extractProjectProfileLocal(repoId: string, repoPath: string): Promise<ProjectProfile> {
  const repoName = repoPath.split("/").pop() ?? repoId;

  // Get top symbol names from Kuzu
  const { db, conn } = await getOrCreateDb(repoId);
  let keySymbols: string[] = [];
  let dominantLanguage = "typescript";
  try {
    const project = await resolveProject(repoPath, repoId);
    // Use a broad query to get any symbols — empty string matches nothing in CONTAINS,
    // so we search for a common short token
    const syms = await searchSymbols(conn, "", project, 20);
    keySymbols = syms.map(s => s.name).filter(Boolean).slice(0, 20);
  } catch {
    // ignore — kuzu may not be indexed yet
  } finally {
    await conn.close();
    await db.close();
  }

  // Read package.json
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

  // Language detection from deps
  if ("typescript" in allDeps || "@types/node" in allDeps) dominantLanguage = "typescript";
  else if ("python" in allDeps) dominantLanguage = "python";

  // Package names from workspaces
  let packageNames: string[] = [];
  if (pkg.workspaces) {
    const ws = Array.isArray(pkg.workspaces) ? pkg.workspaces : pkg.workspaces.packages;
    packageNames = ws.map((p: string) => p.replace(/^packages\//, "").replace(/\/\*$/, ""));
  }

  // Test pattern
  const testPattern = "vitest" in allDeps ? "*.spec.ts" : "jest" in allDeps ? "*.test.ts" : "*.spec.ts";

  // Read .sensei/config.yaml (best-effort)
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
    keySymbols,
    testPattern,
    cliCommands,
    senseiConfig,
  };
}

export async function installSkillsTool(
  repoId: string,
  repoPath: string,
): Promise<{ filesWritten: string[]; errors: string[] }> {
  try {
    const backend = new ClaudeBackend();
    await backend.init();

    const profile = await extractProjectProfileLocal(repoId, repoPath);
    const validator = new SkillValidator(backend, profile);
    const generator = new SkillGenerator(backend, profile, validator);
    const skills = await generator.generate();

    const adapter = new ClaudeAdapter();
    const repoSlug = profile.repoName.toLowerCase().replace(/[^a-z0-9]+/g, "-");
    const written = await adapter.writeSkills(skills, repoSlug);

    // Write manifest so dashboard reflects updated state
    const manifest: AgentSkillsManifest = {
      agent: "claude",
      repoSlug,
      skills: written,
      updatedAt: new Date().toISOString(),
    };
    const senseiDir = join(repoPath, ".sensei");
    await mkdir(senseiDir, { recursive: true });
    await writeFile(join(senseiDir, "agent-skills.json"), JSON.stringify(manifest, null, 2), "utf-8");

    return { filesWritten: written.map(f => f.path), errors: [] };
  } catch (err) {
    return { filesWritten: [], errors: [err instanceof Error ? err.message : String(err)] };
  }
}
