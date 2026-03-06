import { readFile, writeFile, mkdir, stat } from "fs/promises";
import { join } from "path";
import { existsSync } from "fs";
import { execSync } from "child_process";
import yaml from "js-yaml";
import fg from "fast-glob";
import type { SymbolMap } from "../types.js";

const ALWAYS_IGNORE = ["**/.git/**", "**/.index/**"];
const CODE_EXTS = [".ts", ".tsx", ".js", ".jsx", ".py", ".go", ".rs"];
const DOC_EXTS = [".md", ".mdx", ".txt", ".yaml", ".yml"];

async function loadGitignorePatterns(repoPath: string): Promise<string[]> {
  const gitignorePath = join(repoPath, ".gitignore");
  if (!existsSync(gitignorePath)) return [];
  const content = await readFile(gitignorePath, "utf-8");
  return content
    .split("\n")
    .map(l => l.trim())
    .filter(l => l && !l.startsWith("#"))
    .flatMap(pattern => {
      // Convert gitignore patterns to fast-glob ignore patterns
      const p = pattern.replace(/^\//, ""); // strip leading slash
      if (p.includes("/")) return [`**/${p}`, `**/${p}/**`];
      return [`**/${p}`, `**/${p}/**`];
    });
}

export interface IndexSummary {
  added: number;
  updated: number;
  removed: number;
  unchanged: number;
  forced: boolean;
}

interface DocIndexData {
  lastIndexedCommit?: string;
  files: Record<string, { mtime: number; size: number }>;
}

export async function reindexRepo(
  repoPath: string,
  options?: { force?: boolean }
): Promise<IndexSummary> {
  await mkdir(join(repoPath, ".index"), { recursive: true });

  const gitignorePatterns = await loadGitignorePatterns(repoPath);
  const IGNORE = [...ALWAYS_IGNORE, ...gitignorePatterns];

  const docIndexPath = join(repoPath, ".index/doc-index.json");
  const symbolMapPath = join(repoPath, ".index/symbol-map.json");

  // Load existing state
  let existingDocIndex: DocIndexData | null = null;
  let existingSymbolMap: SymbolMap = {};

  if (existsSync(docIndexPath) && existsSync(symbolMapPath)) {
    try {
      existingDocIndex = JSON.parse(await readFile(docIndexPath, "utf-8"));
      // Handle old format (flat object without files key)
      if (existingDocIndex && !existingDocIndex.files) {
        existingDocIndex = { files: existingDocIndex as unknown as Record<string, { mtime: number; size: number }> };
      }
      existingSymbolMap = JSON.parse(await readFile(symbolMapPath, "utf-8"));
    } catch {
      existingDocIndex = null;
    }
  }

  const force = options?.force ?? existingDocIndex === null;

  // Determine changed files
  const isGit = existsSync(join(repoPath, ".git"));
  const lastCommit = existingDocIndex?.lastIndexedCommit;
  let changedFiles = new Set<string>();
  let deletedFiles = new Set<string>();

  if (!force && isGit && lastCommit) {
    try {
      const changed = execSync(`git diff ${lastCommit}..HEAD --name-only`, { cwd: repoPath })
        .toString().trim().split("\n").filter(Boolean);
      const deleted = execSync(`git diff ${lastCommit}..HEAD --name-only --diff-filter=D`, { cwd: repoPath })
        .toString().trim().split("\n").filter(Boolean);
      changedFiles = new Set(changed);
      deletedFiles = new Set(deleted);
    } catch {
      // git diff failed (shallow clone, etc.) — fall through to mtime fallback
    }
  }

  // Glob all current files
  const codeFiles = await fg(CODE_EXTS.map(e => `**/*${e}`), {
    cwd: repoPath, ignore: IGNORE, absolute: false,
  });
  const docFiles = await fg(DOC_EXTS.map(e => `**/*${e}`), {
    cwd: repoPath, ignore: IGNORE, absolute: false,
  });
  const allCurrentFiles = new Set([...codeFiles, ...docFiles]);

  // Build updated symbol map
  const symbolMap: SymbolMap = { ...existingSymbolMap };
  let added = 0, updated = 0, removed = 0, unchanged = 0;

  // Remove deleted files from symbol map
  for (const file of Object.keys(symbolMap)) {
    if (!allCurrentFiles.has(file) || deletedFiles.has(file)) {
      delete symbolMap[file];
      removed++;
    }
  }

  // Process code files
  for (const file of codeFiles) {
    if (deletedFiles.has(file)) continue;

    let needsExtraction = force;

    if (!needsExtraction) {
      if (changedFiles.size > 0) {
        // Git mode: process only files in git diff
        needsExtraction = changedFiles.has(file);
      } else {
        // Mtime fallback
        const stored = existingDocIndex?.files[file];
        if (!stored) {
          needsExtraction = true;
        } else {
          const s = await stat(join(repoPath, file));
          needsExtraction = Math.abs(s.mtimeMs - stored.mtime) > 1000 || s.size !== stored.size;
        }
      }
    }

    if (needsExtraction) {
      const content = await readFile(join(repoPath, file), "utf-8");
      const exports = extractExports(content);
      if (exports.L0.length > 0) {
        const isNew = !(file in existingSymbolMap);
        symbolMap[file] = exports;
        if (isNew) added++; else updated++;
      }
    } else {
      unchanged++;
    }
  }

  // Build doc-index fingerprints for all current files
  const fingerprints: Record<string, { mtime: number; size: number }> = {};
  await Promise.all([...allCurrentFiles].map(async (file) => {
    const s = await stat(join(repoPath, file));
    fingerprints[file] = { mtime: s.mtimeMs, size: s.size };
  }));

  // Get current HEAD commit
  let currentCommit: string | undefined;
  if (isGit) {
    try {
      currentCommit = execSync("git rev-parse HEAD", { cwd: repoPath }).toString().trim();
    } catch { /* not a git repo or no commits */ }
  }

  const newDocIndex: DocIndexData = {
    ...(currentCommit ? { lastIndexedCommit: currentCommit } : {}),
    files: fingerprints,
  };

  // Build traceability matrix from .llmspec.yaml
  const traceability = await buildTraceability(repoPath, docFiles);

  // Write all artifacts
  const [stack, shortcuts] = await Promise.all([
    detectStack(repoPath),
    detectShortcuts(repoPath),
  ]);

  await Promise.all([
    writeFile(join(repoPath, ".index/stack.md"), formatStack(stack)),
    writeFile(join(repoPath, ".index/shortcuts.md"), formatShortcuts(shortcuts)),
    writeFile(join(repoPath, ".index/symbol-map.json"), JSON.stringify(symbolMap, null, 2)),
    writeFile(join(repoPath, ".index/doc-index.json"), JSON.stringify(newDocIndex, null, 2)),
    writeFile(join(repoPath, ".index/traceability.json"), JSON.stringify(traceability, null, 2)),
    ensurePatternsMd(repoPath),
  ]);

  if (!existsSync(join(repoPath, ".llmspec.yaml"))) {
    await writeFile(join(repoPath, ".llmspec.yaml"), generateLlmSpecTemplate(repoPath, stack, shortcuts));
  }

  await Promise.all([
    generateLlmsTxt(repoPath),
    generateClaudeMd(repoPath),
  ]);

  return { added, updated, removed, unchanged, forced: force };
}

async function buildTraceability(
  repoPath: string,
  docFiles: string[]
): Promise<Record<string, string[]>> {
  const result: Record<string, string[]> = {};

  // Load manual declarations from .llmspec.yaml
  const llmspecPath = join(repoPath, ".llmspec.yaml");
  if (existsSync(llmspecPath)) {
    try {
      const spec = yaml.load(await readFile(llmspecPath, "utf-8")) as Record<string, unknown>;
      const docs = spec?.docs as Array<{ path: string; covers?: string[] }> | undefined;
      if (Array.isArray(docs)) {
        for (const entry of docs) {
          if (entry.path && Array.isArray(entry.covers)) {
            result[entry.path] = entry.covers;
          }
        }
      }
    } catch { /* malformed yaml */ }
  }

  // Auto-detect: scan doc content for filename references
  for (const docFile of docFiles) {
    if (result[docFile]) continue; // manual entry takes precedence
    try {
      const content = await readFile(join(repoPath, docFile), "utf-8");
      const srcRefs = Array.from(
        new Set(
          Array.from(content.matchAll(/\bsrc\/[\w/.-]+\.[a-z]+\b/g)).map(m => m[0])
        )
      );
      if (srcRefs.length > 0) result[docFile] = srcRefs;
    } catch { /* skip unreadable */ }
  }

  return result;
}

async function ensurePatternsMd(repoPath: string): Promise<void> {
  const path = join(repoPath, ".index/patterns.md");
  if (!existsSync(path)) {
    await writeFile(path, "# Patterns\n\n<!-- Review and expand -->\n");
  }
}

async function detectStack(repoPath: string): Promise<Record<string, string[]>> {
  const stack: Record<string, string[]> = { languages: [], frameworks: [], tools: [] };
  const pkgPath = join(repoPath, "package.json");
  if (existsSync(pkgPath)) {
    const pkg = JSON.parse(await readFile(pkgPath, "utf-8"));
    stack.languages.push("typescript/javascript");
    const deps = { ...pkg.dependencies, ...pkg.devDependencies };
    ["react", "next", "vue", "svelte", "express", "fastify", "hono"].forEach(f => {
      if (deps[f]) stack.frameworks.push(f);
    });
    if (deps["vitest"]) stack.tools.push("vitest");
    if (deps["jest"]) stack.tools.push("jest");
    if (deps["bun"]) stack.tools.push("bun");
  }
  if (existsSync(join(repoPath, "pyproject.toml")) || existsSync(join(repoPath, "requirements.txt"))) {
    stack.languages.push("python");
  }
  return stack;
}

async function detectShortcuts(repoPath: string): Promise<Record<string, string>> {
  const pkgPath = join(repoPath, "package.json");
  if (existsSync(pkgPath)) {
    const pkg = JSON.parse(await readFile(pkgPath, "utf-8"));
    return pkg.scripts ?? {};
  }
  return {};
}

export function extractExports(content: string): { L0: string[]; L1: string[]; L2: string[] } {
  const L0: string[] = [];
  const exportRe = /^export\s+(async\s+)?(function|class|const|type|interface|enum)\s+(\w+[^{;=\n]*)/gm;
  let m: RegExpExecArray | null;
  while ((m = exportRe.exec(content)) !== null) {
    const sig = m[0].replace(/\{.*/, "").replace(/=\s*$/, "").trim();
    L0.push(sig);
  }
  return { L0, L1: L0.map(s => `// ${s}`), L2: [] };
}

function formatStack(stack: Record<string, string[]>): string {
  return "# Tech Stack\n\n" +
    Object.entries(stack)
      .filter(([, v]) => v.length)
      .map(([k, v]) => `## ${k}\n${v.map(x => `- ${x}`).join("\n")}`)
      .join("\n\n");
}

function formatShortcuts(shortcuts: Record<string, string>): string {
  return "# Shortcuts\n\n" +
    Object.entries(shortcuts).map(([k, v]) => `- **${k}**: \`${v}\``).join("\n");
}

function generateLlmSpecTemplate(repoPath: string, stack: Record<string, string[]>, shortcuts: Record<string, string>): string {
  const name = repoPath.split("/").at(-1) ?? "project";
  return yaml.dump({
    project: name,
    version: "0.0.0",
    description: "TODO: one-sentence project summary",
    stack: [...stack.languages, ...stack.frameworks],
    entry_points: [{ path: "src/index.ts", role: "TODO: describe role" }],
    concepts: [],
    patterns: [],
    api_surface: [],
    doc_layers: { design: "docs/", code: "src/", public: ["README.md"] },
    docs: [],
    shortcuts,
  });
}

async function generateLlmsTxt(repoPath: string): Promise<void> {
  const readmePath = join(repoPath, "README.md");
  const readme = existsSync(readmePath) ? await readFile(readmePath, "utf-8") : "";
  const name = repoPath.split("/").at(-1) ?? "project";
  await writeFile(join(repoPath, "llms.txt"),
    `# ${name}\n\n${readme.split("\n").slice(0, 20).join("\n")}\n\n> Generated by sensei\n`);
}

async function generateClaudeMd(repoPath: string): Promise<void> {
  const claudeMdPath = join(repoPath, "CLAUDE.md");
  if (existsSync(claudeMdPath)) return;
  await writeFile(claudeMdPath,
    `# Project Context\n\n> Auto-generated by sensei. Update as needed.\n\n` +
    `## Orientation\nCall \`get_session_context()\` to resume a session.\n` +
    `Call \`get_llmspec()\` for full repo orientation.\n\n` +
    `## Stack\nSee \`.index/stack.md\`\n\n` +
    `## Shortcuts\nSee \`.index/shortcuts.md\`\n\n` +
    `## Patterns\nSee \`.index/patterns.md\`\n`);
}
