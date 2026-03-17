import { readFile } from "fs/promises";
import { existsSync } from "fs";
import yaml from "js-yaml";
import type { LlmSpec, SymbolMap } from "@sensei/shared";
import { SENSEI_DIR, senseiPath, loadSenseiConfig, makeSenseiClient } from "@sensei/shared";

export async function readLlmSpec(repoPath: string): Promise<LlmSpec> {
  const specPath = senseiPath(repoPath, "llmspec.yaml");
  let raw: string;
  try {
    raw = await readFile(specPath, "utf-8");
  } catch {
    throw new Error(`No ${SENSEI_DIR}/llmspec.yaml found at ${specPath}. Run sensei index first.`);
  }
  return yaml.load(raw) as LlmSpec;
}

export async function readSymbolMap(repoPath: string): Promise<SymbolMap> {
  // Try DB first
  const config = await loadSenseiConfig(repoPath);
  if (config) {
    const client = await makeSenseiClient(repoPath);
    if (client) {
      const { data } = await (client as any)
        .schema("sensei").from("symbol_map")
        .select("file_path, l0, l1")
        .eq("repo_id", config.repo_id);
      if (data && data.length > 0) {
        const map: SymbolMap = {};
        for (const row of data) {
          map[row.file_path] = {
            L0: row.l0,
            L1: typeof row.l1 === "string" ? row.l1.split("\n").filter(Boolean) : (row.l1 ?? []),
            L2: [],
          };
        }
        return map;
      }
    }
  }
  // Fallback to file
  try {
    const raw = await readFile(senseiPath(repoPath, "symbol-map.json"), "utf-8");
    return JSON.parse(raw) as SymbolMap;
  } catch {
    return {};
  }
}

export async function readIndexFile(repoPath: string, filename: string): Promise<string | null> {
  // For stack.md and shortcuts.md, try DB first
  const dbColumn = filename === "stack.md" ? "stack_md" : filename === "shortcuts.md" ? "shortcuts_md" : null;
  if (dbColumn) {
    const config = await loadSenseiConfig(repoPath);
    if (config) {
      const client = await makeSenseiClient(repoPath);
      if (client) {
        const { data } = await (client as any)
          .schema("sensei").from("repos")
          .select(dbColumn)
          .eq("id", config.repo_id)
          .maybeSingle();
        if (data?.[dbColumn]) return data[dbColumn];
      }
    }
  }
  // Fallback to file (patterns.md always reads from file)
  try {
    return await readFile(senseiPath(repoPath, filename), "utf-8");
  } catch {
    return null;
  }
}

