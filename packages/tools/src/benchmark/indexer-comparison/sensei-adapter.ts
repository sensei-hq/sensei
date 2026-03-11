import { readFile } from "fs/promises";
import { existsSync } from "fs";
import { join } from "path";
import type { SenseiSymbol } from "./types.js";

interface SymbolMapEntry {
  L0: string[];
  L1?: string[];
}

export interface SenseiIndex {
  symbols: SenseiSymbol[];
  files: string[];
  missing: boolean;
}

export async function loadSenseiIndex(repoPath: string): Promise<SenseiIndex> {
  const symbolMapPath = join(repoPath, ".sensei", "symbol-map.json");
  if (!existsSync(symbolMapPath)) {
    return { symbols: [], files: [], missing: true };
  }

  const raw: Record<string, SymbolMapEntry> = JSON.parse(
    await readFile(symbolMapPath, "utf-8")
  );

  const symbols: SenseiSymbol[] = [];
  const files = Object.keys(raw);

  for (const [path, entry] of Object.entries(raw)) {
    for (let i = 0; i < entry.L0.length; i++) {
      const sig = entry.L0[i];
      // Extract symbol name from signature like "export function reindexRepo"
      const match = sig.match(/(?:function|class|const|type|interface)\s+(\w+)/);
      if (!match) continue;
      symbols.push({
        name: match[1],
        path,
        L0: sig,
        L1: entry.L1?.[i],
      });
    }
  }

  return { symbols, files, missing: false };
}
