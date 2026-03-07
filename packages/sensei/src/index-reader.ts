import { readFile } from "fs/promises";
import yaml from "js-yaml";
import type { LlmSpec, SymbolMap } from "@sensei/shared";
import { SENSEI_DIR, senseiPath } from "@sensei/shared";

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
  const raw = await readFile(senseiPath(repoPath, "symbol-map.json"), "utf-8");
  return JSON.parse(raw) as SymbolMap;
}

export async function readIndexFile(repoPath: string, filename: string): Promise<string | null> {
  try {
    return await readFile(senseiPath(repoPath, filename), "utf-8");
  } catch {
    return null;
  }
}
