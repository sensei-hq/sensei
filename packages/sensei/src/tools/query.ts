import { readFile } from "fs/promises";
import { join } from "path";
import yaml from "js-yaml";
import { readLlmSpec, readSymbolMap, readIndexFile } from "../index-reader.js";
import type { ResolutionLevel } from "../types.js";

export async function getLlmSpec(repoPath: string, section?: string): Promise<string> {
  const spec = await readLlmSpec(repoPath);
  if (!section) return yaml.dump(spec);
  const value = (spec as Record<string, unknown>)[section];
  if (value === undefined) throw new Error(`Section '${section}' not found in llmspec`);
  return yaml.dump({ [section]: value });
}

export async function getFileContext(repoPath: string, filePath: string, level: ResolutionLevel): Promise<string> {
  if (level === "L3") {
    return readFile(join(repoPath, filePath), "utf-8");
  }
  const map = await readSymbolMap(repoPath);
  const entry = map[filePath];
  if (!entry) throw new Error(`File '${filePath}' not in symbol map. Run reindex_repo first.`);
  return entry[level].join("\n");
}

export async function listExports(repoPath: string, module?: string): Promise<string> {
  const map = await readSymbolMap(repoPath);
  const lines: string[] = [];
  for (const [file, entry] of Object.entries(map)) {
    if (module && !file.startsWith(module)) continue;
    lines.push(`\n### ${file}`);
    lines.push(...entry.L0);
  }
  return lines.join("\n");
}

export async function findPattern(repoPath: string, name?: string): Promise<string> {
  const content = await readIndexFile(repoPath, "patterns.md");
  if (!content) return "No patterns indexed. Run reindex_repo first.";
  if (!name) return content;
  const lines = content.split("\n");
  const start = lines.findIndex(l => l.toLowerCase().includes(name.toLowerCase()));
  if (start === -1) return `Pattern '${name}' not found.`;
  const end = lines.findIndex((l, i) => i > start && l.startsWith("## "));
  return lines.slice(start, end === -1 ? undefined : end).join("\n");
}

export async function getShortcuts(repoPath: string): Promise<string> {
  const content = await readIndexFile(repoPath, "shortcuts.md");
  if (!content) {
    const spec = await readLlmSpec(repoPath);
    return yaml.dump(spec.shortcuts);
  }
  return content;
}
