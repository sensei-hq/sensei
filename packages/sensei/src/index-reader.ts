import { readFile } from "fs/promises";
import { join } from "path";
import yaml from "js-yaml";
import type { LlmSpec, SymbolMap } from "./types.js";

export async function readLlmSpec(repoPath: string): Promise<LlmSpec> {
  const specPath = join(repoPath, ".index/llmspec.yaml");
  let raw: string;
  try {
    raw = await readFile(specPath, "utf-8");
  } catch {
    throw new Error(`No .index/llmspec.yaml found at ${specPath}. Run sensei index first.`);
  }
  return yaml.load(raw) as LlmSpec;
}

export async function readSymbolMap(repoPath: string): Promise<SymbolMap> {
  const mapPath = join(repoPath, ".index/symbol-map.json");
  const raw = await readFile(mapPath, "utf-8");
  return JSON.parse(raw) as SymbolMap;
}

export async function readIndexFile(repoPath: string, filename: string): Promise<string | null> {
  try {
    return await readFile(join(repoPath, ".index", filename), "utf-8");
  } catch {
    return null;
  }
}
