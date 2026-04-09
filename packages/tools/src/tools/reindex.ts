import { readFile, writeFile, mkdir, stat } from "fs/promises";
import { join } from "path";
import { existsSync } from "fs";
import { execSync } from "child_process";
import yaml from "js-yaml";
import fg from "fast-glob";
import type { SymbolMap, LlmSpec, LlmsTxtSection } from "@sensei/shared";
import { SENSEI_DIR, senseiPath } from "@sensei/shared";
import { inferEntryPoints } from "./entry-point-adapters.js";
import type { EntryPointCandidate } from "./entry-point-adapters.js";
import { generateLlmsTxt as buildLlmsTxt } from "./llms-txt.js";

const ALWAYS_IGNORE = ["**/.git/**", `**/${SENSEI_DIR}/**`, "CLAUDE.md"];
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
  skipped: number;   // scanned but no symbols found
  total: number;     // total files scanned (code + markdown)
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
  await mkdir(join(repoPath, SENSEI_DIR), { recursive: true });

  const gitignorePatterns = await loadGitignorePatterns(repoPath);
  const IGNORE = [...ALWAYS_IGNORE, ...gitignorePatterns];

  // Load existing state from local doc-index.json
  let existingDocIndex: DocIndexData | null = null;
  let existingSymbolMap: SymbolMap = {};

  const docIndexPath = join(repoPath, SENSEI_DIR, "doc-index.json");
  if (existsSync(docIndexPath)) {
    try {
      existingDocIndex = JSON.parse(await readFile(docIndexPath, "utf-8")) as DocIndexData;
    } catch { /* start fresh */ }
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
  let added = 0, updated = 0, removed = 0, unchanged = 0, skipped = 0;

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
        // In git diff mode, still extract files not yet in symbol map
        needsExtraction = changedFiles.has(file) || !(file in existingSymbolMap);
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
      const result = exports.L0.length > 0 ? exports : extractHeuristic(content, file);
      if (result.L0.length > 0) {
        const isNew = !(file in existingSymbolMap);
        symbolMap[file] = result;
        if (isNew) added++; else updated++;
      } else {
        skipped++;
      }
    } else {
      unchanged++;
    }
  }

  // Process markdown doc files — add L0/L1 to symbol-map
  for (const file of docFiles) {
    if (!file.endsWith(".md") && !file.endsWith(".mdx")) continue;
    if (deletedFiles.has(file)) continue;

    let needsExtraction = force;

    if (!needsExtraction) {
      if (changedFiles.size > 0) {
        // In git diff mode, still extract files not yet in symbol map
        needsExtraction = changedFiles.has(file) || !(file in existingSymbolMap);
      } else {
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
      const symbols = extractMarkdownSymbols(content, file);
      const isNew = !(file in existingSymbolMap);
      symbolMap[file] = symbols;
      if (isNew) added++; else updated++;
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

  // Write all artifacts
  const [stack, shortcuts] = await Promise.all([
    detectStack(repoPath),
    detectShortcuts(repoPath),
  ]);

  await ensurePatternsMd(repoPath);

  // Read existing description for llms.txt header
  let existingDescription = "";
  const llmspecPath = senseiPath(repoPath, "llmspec.yaml");
  if (existsSync(llmspecPath)) {
    try {
      const existing = yaml.load(await readFile(llmspecPath, "utf-8")) as any;
      existingDescription = existing?.description ?? "";
    } catch { /* ignore */ }
  }

  // Infer entry points
  const inferredEntryPoints = await inferEntryPoints(repoPath);

  const specForHeader: LlmSpec = {
    project: repoPath.split("/").at(-1) ?? "project",
    version: "0.0.0",
    description: existingDescription || "TODO: one-sentence project summary",
    stack: [...stack.languages, ...stack.frameworks],
    entry_points: inferredEntryPoints.map(e => ({ path: e.path, role: e.inferredRole })),
    concepts: [],
    patterns: [],
    api_surface: [],
    doc_layers: { design: "docs/", code: "src/", public: ["README.md"] },
    shortcuts,
    llms_txt: [],
  };

  const { sections: llmsTxtSections } = await buildLlmsTxt(repoPath, specForHeader, symbolMap, changedFiles);

  await mergeLlmSpec(repoPath, {
    stack: [...stack.languages, ...stack.frameworks],
    shortcuts,
    version: "0.0.0",
    entryPoints: inferredEntryPoints,
    llmsTxtSections,
  });

  await generateClaudeMd(repoPath);

  const total = added + updated + unchanged + skipped;

  // Persist local doc-index.json fingerprints for next run
  await writeFile(docIndexPath, JSON.stringify(newDocIndex, null, 2));

  return { added, updated, removed, unchanged, skipped, total, forced: force };
}

async function ensurePatternsMd(repoPath: string): Promise<void> {
  // patterns.md lives at the repo root — user-editable, committed to git
  const path = join(repoPath, "PATTERNS.md");
  if (!existsSync(path)) {
    await writeFile(path, "# Patterns\n\n<!-- Document recurring code patterns and conventions for this project. -->\n");
  }
}

async function findPackageJson(repoPath: string): Promise<Record<string, unknown> | null> {
  // Check root first, then immediate subdirectories (monorepo support)
  const candidates = [
    join(repoPath, "package.json"),
    ...["src", "app", "solution", "packages", "web", "api", "server", "client"]
      .map(d => join(repoPath, d, "package.json")),
  ];
  for (const p of candidates) {
    if (existsSync(p)) {
      try { return JSON.parse(await readFile(p, "utf-8")); } catch { /* skip */ }
    }
  }
  // Fall back to any package.json one level deep
  try {
    const entries = await fg(["*/package.json"], { cwd: repoPath, deep: 1, ignore: ALWAYS_IGNORE });
    for (const rel of entries.slice(0, 3)) {
      try { return JSON.parse(await readFile(join(repoPath, rel), "utf-8")); } catch { /* skip */ }
    }
  } catch { /* skip */ }
  return null;
}

async function detectStack(repoPath: string): Promise<Record<string, string[]>> {
  const stack: Record<string, string[]> = { languages: [], frameworks: [], tools: [] };
  const pkg = await findPackageJson(repoPath);
  if (pkg) {
    stack.languages.push("typescript/javascript");
    const deps: Record<string, string> = { ...(pkg as any).dependencies, ...(pkg as any).devDependencies };
    const FRAMEWORKS = ["react", "next", "@sveltejs/kit", "svelte", "vue", "nuxt", "solid-js",
      "express", "fastify", "hono", "koa", "elysia", "convex", "trpc", "drizzle-orm", "prisma"];
    FRAMEWORKS.forEach(f => { if (deps[f]) stack.frameworks.push(f.replace(/^@[^/]+\//, "")); });
    const TOOLS = ["vitest", "jest", "playwright", "storybook", "turbo", "nx", "bun", "vite", "esbuild"];
    TOOLS.forEach(t => { if (deps[t] || deps[`@${t}/core`]) stack.tools.push(t); });
  }
  if (existsSync(join(repoPath, "pyproject.toml")) || existsSync(join(repoPath, "requirements.txt"))) {
    stack.languages.push("python");
  }
  if (existsSync(join(repoPath, "go.mod"))) stack.languages.push("go");
  if (existsSync(join(repoPath, "Cargo.toml"))) stack.languages.push("rust");
  return stack;
}

async function detectShortcuts(repoPath: string): Promise<Record<string, string>> {
  const pkg = await findPackageJson(repoPath);
  return (pkg as any)?.scripts ?? {};
}

export function extractExports(content: string): { L0: string[]; L1: string[]; L2: string[] } {
  const L0: string[] = [];
  const L1: string[] = [];
  const lines = content.split("\n");
  const exportRe = /^export\s+(async\s+)?(function|class|const|type|interface|enum)\s+(\w+[^{;=\n]*)/;

  for (let i = 0; i < lines.length; i++) {
    const m = exportRe.exec(lines[i]);
    if (!m) continue;

    const sig = m[0].replace(/\{.*/, "").replace(/=\s*$/, "").trim();
    L0.push(sig);

    // Look backwards for JSDoc (/** ... */) — skip blank lines first
    let j = i - 1;
    while (j >= 0 && lines[j].trim() === "") j--;

    let description = "";
    if (j >= 0 && /^\/\*\*.*\*\/\s*$/.test(lines[j].trim())) {
      // Single-line JSDoc: /** description */
      description = lines[j].replace(/^\s*\/\*\*\s*/, "").replace(/\s*\*\/\s*$/, "").trim().slice(0, 120);
    } else if (j >= 0 && lines[j].trim() === "*/") {
      // Multi-line JSDoc: last line is */
      j--;
      const docLines: string[] = [];
      while (j >= 0 && !lines[j].includes("/**")) {
        const stripped = lines[j].replace(/^\s*\*\s?/, "").trim();
        if (stripped && !stripped.startsWith("@")) docLines.unshift(stripped);
        j--;
      }
      description = docLines.join(" ").trim().slice(0, 120);
    }
    L1.push(description ? `// ${description}\n// ${sig}` : `// ${sig}`);
  }

  return { L0, L1, L2: [] };
}

export function extractMarkdownSymbols(content: string, filename: string): { L0: string[]; L1: string[]; L2: string[] } {
  const lines = content.split("\n");

  // L0: title (H1 or filename) + first substantive paragraph (≤150 chars)
  const h1 = lines.find(l => l.startsWith("# "))?.replace(/^# /, "").trim() ?? filename.replace(/\.mdx?$/, "");
  const bodyLines = lines.filter(l => !l.startsWith("#") && l.trim().length > 0);
  const firstPara = bodyLines.slice(0, 3).join(" ").replace(/\s+/g, " ").trim().slice(0, 150);
  const l0Entry = firstPara ? `${h1} — ${firstPara}` : h1;

  // L1: L0 prefixed with //, then H2/H3 section headings prefixed with //
  const sections = lines.filter(l => /^#{2,3} /.test(l)).map(l => `// ${l.trim()}`);
  const l1 = [`// ${l0Entry}`, ...sections];

  return { L0: [l0Entry], L1: l1, L2: [] };
}

export function extractHeuristic(content: string, filename: string): { L0: string[]; L1: string[]; L2: string[] } {
  const L0: string[] = [];
  const ext = filename.split(".").at(-1) ?? "";
  const lines = content.split("\n");

  for (const line of lines) {
    let m: RegExpExecArray | null = null;
    if (["ts", "tsx", "js", "jsx"].includes(ext)) {
      m = /^(?:async\s+)?function\s+\w[^({]*/.exec(line)
        ?? /^(?:const|let)\s+\w+\s*=\s*(?:async\s*)?\(/.exec(line);
    } else if (ext === "py") {
      m = /^(?:async\s+)?def\s+\w[^:]*:/.exec(line)
        ?? /^class\s+\w[^:]*:/.exec(line);
    } else if (ext === "go") {
      m = /^func\s+(?:\(\w[^)]*\)\s+)?\w[^({]*/.exec(line);
    } else if (ext === "rs") {
      m = /^(?:pub\s+(?:async\s+)?)?fn\s+\w[^({]*/.exec(line);
    }
    if (m) L0.push(m[0].replace(/\s*\{.*/, "").trim());
  }

  return { L0, L1: L0.map(s => `// ${s}`), L2: [] };
}

export async function mergeLlmSpec(
  repoPath: string,
  structural: {
    stack: string[];
    shortcuts: Record<string, string>;
    version: string;
    entryPoints: EntryPointCandidate[];
    llmsTxtSections: LlmsTxtSection[];
  }
): Promise<void> {
  const llmspecPath = senseiPath(repoPath, "llmspec.yaml");

  if (!existsSync(llmspecPath)) {
    await mkdir(join(repoPath, SENSEI_DIR), { recursive: true });
    const name = repoPath.split("/").at(-1) ?? "project";
    const spec: LlmSpec = {
      project: name,
      version: structural.version,
      description: "TODO: one-sentence project summary",
      stack: structural.stack,
      entry_points: structural.entryPoints.map(e => ({ path: e.path, role: e.inferredRole })),
      concepts: [],
      patterns: [],
      api_surface: [],
      doc_layers: { design: "docs/", code: "src/", public: ["README.md"] },
      shortcuts: structural.shortcuts,
      llms_txt: structural.llmsTxtSections,
    };
    await writeFile(llmspecPath, yaml.dump(spec));
    return;
  }

  let existing: Record<string, unknown>;
  try {
    existing = yaml.load(await readFile(llmspecPath, "utf-8")) as Record<string, unknown>;
  } catch {
    return;
  }

  const existingEPs: Array<{ path: string; role: string }> =
    (existing.entry_points as Array<{ path: string; role: string }>) ?? [];
  const existingRoleMap = new Map(existingEPs.map(e => [e.path, e.role]));
  const newEPs = structural.entryPoints.map(c => ({
    path: c.path,
    role: existingRoleMap.get(c.path) ?? c.inferredRole,
  }));

  const merged: Record<string, unknown> = {
    ...existing,
    stack: structural.stack,
    shortcuts: structural.shortcuts,
    version: structural.version,
    entry_points: newEPs,
    llms_txt: structural.llmsTxtSections,
  };

  await writeFile(llmspecPath, yaml.dump(merged));
}

async function generateClaudeMd(repoPath: string): Promise<void> {
  const claudeMdPath = join(repoPath, "CLAUDE.md");
  if (existsSync(claudeMdPath)) return;
  await writeFile(claudeMdPath,
    `# Project Context\n\n> Auto-generated by sensei. Update as needed.\n\n` +
    `## Orientation\nCall \`get_session_context()\` to resume a session.\n` +
    `Call \`get_llmspec()\` for full repo orientation.\n\n` +
    `## Stack\nCall \`get_session_context()\` — stack is stored in Supabase.\n\n` +
    `## Shortcuts\nCall \`get_session_context()\` — shortcuts are stored in Supabase.\n\n` +
    `## Patterns\nSee \`PATTERNS.md\` at the project root.\n`);
}
