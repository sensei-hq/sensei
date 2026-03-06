import { readFile, writeFile, mkdir, stat } from "fs/promises";
import { join } from "path";
import { existsSync } from "fs";
import yaml from "js-yaml";
import fg from "fast-glob";
import type { SymbolMap } from "../types.js";

const IGNORE = ["node_modules", "dist", ".git", "coverage", ".cache", ".index"];
const CODE_EXTS = [".ts", ".tsx", ".js", ".jsx", ".py", ".go", ".rs"];

export async function reindexRepo(repoPath: string): Promise<void> {
  await mkdir(join(repoPath, ".index"), { recursive: true });

  const [stack, shortcuts, symbolMap, docIndex] = await Promise.all([
    detectStack(repoPath),
    detectShortcuts(repoPath),
    buildSymbolMap(repoPath),
    buildDocIndex(repoPath),
  ]);

  await Promise.all([
    writeFile(join(repoPath, ".index/stack.md"), formatStack(stack)),
    writeFile(join(repoPath, ".index/shortcuts.md"), formatShortcuts(shortcuts)),
    writeFile(join(repoPath, ".index/symbol-map.json"), JSON.stringify(symbolMap, null, 2)),
    writeFile(join(repoPath, ".index/doc-index.json"), JSON.stringify(docIndex, null, 2)),
    writeFile(join(repoPath, ".index/patterns.md"), "# Patterns\n\n<!-- Review and expand -->\n"),
  ]);

  if (!existsSync(join(repoPath, ".llmspec.yaml"))) {
    await writeFile(join(repoPath, ".llmspec.yaml"), generateLlmSpecTemplate(repoPath, stack, shortcuts));
  }

  await Promise.all([
    generateLlmsTxt(repoPath),
    generateClaudeMd(repoPath),
  ]);
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

async function buildSymbolMap(repoPath: string): Promise<SymbolMap> {
  const files = await fg(CODE_EXTS.map(e => `**/*${e}`), {
    cwd: repoPath, ignore: IGNORE, absolute: false,
  });
  const map: SymbolMap = {};
  await Promise.all(files.map(async (file) => {
    const content = await readFile(join(repoPath, file), "utf-8");
    const exports = extractExports(content);
    if (exports.L0.length > 0) map[file] = exports;
  }));
  return map;
}

function extractExports(content: string): { L0: string[]; L1: string[]; L2: string[] } {
  const L0: string[] = [];
  const exportRe = /^export\s+(async\s+)?(function|class|const|type|interface|enum)\s+(\w+[^{;=\n]*)/gm;
  let m: RegExpExecArray | null;
  while ((m = exportRe.exec(content)) !== null) {
    const sig = m[0].replace(/\{.*/, "").replace(/=\s*$/, "").trim();
    L0.push(sig);
  }
  return { L0, L1: L0.map(s => `// ${s}`), L2: [] };
}

async function buildDocIndex(repoPath: string): Promise<Record<string, { mtime: number; size: number }>> {
  const docExts = [".md", ".mdx", ".txt", ".yaml", ".yml"];
  const files = await fg(docExts.map(e => `**/*${e}`), {
    cwd: repoPath, ignore: IGNORE, absolute: false,
  });
  const index: Record<string, { mtime: number; size: number }> = {};
  await Promise.all(files.map(async (file) => {
    const s = await stat(join(repoPath, file));
    index[file] = { mtime: s.mtimeMs, size: s.size };
  }));
  return index;
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
