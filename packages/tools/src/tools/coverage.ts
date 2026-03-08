import { readFile } from "fs/promises";
import { existsSync } from "fs";
import { join } from "path";
import fg from "fast-glob";
import type { ModelBackend, SymbolMap } from "@sensei/shared";
import { SENSEI_DIR, senseiPath } from "@sensei/shared";

export interface CoverageEntry {
  path: string;     // doc file path (relative to repo root)
  covers: string[]; // source file paths it documents
}

const COVERAGE_PROMPT = (
  docPath: string,
  docContent: string,
  sourceFiles: Array<{ path: string; exports: string }>
) =>
  `You are analyzing a documentation file to determine which source code files it documents.

Documentation file: ${docPath}

Documentation content:
---
${docContent.slice(0, 1500)}
---

Available source files (path: key exports):
${sourceFiles.map(f => `- ${f.path}: ${f.exports}`).join("\n")}

Which of these source files does this documentation primarily describe or cover?
Respond with ONLY a JSON array of file paths from the list above.
Example: ["packages/tools/src/tools/reindex.ts"]
If none match, respond with [].`;

/** Extract the first JSON array from a string that may have prose around it. */
function extractJsonArray(text: string): string[] {
  const start = text.indexOf("[");
  const end = text.lastIndexOf("]");
  if (start === -1 || end === -1 || end <= start) return [];
  try {
    const parsed = JSON.parse(text.slice(start, end + 1));
    if (!Array.isArray(parsed)) return [];
    return parsed.filter((x): x is string => typeof x === "string");
  } catch {
    return [];
  }
}

/**
 * Use a local model to determine which source files each doc in docs/ covers.
 * Returns entries suitable for llmspec.yaml docs[].
 */
export async function generateCoverage(
  repoPath: string,
  model: Pick<ModelBackend, "generate">
): Promise<CoverageEntry[]> {
  const symbolMapPath = senseiPath(repoPath, "symbol-map.json");
  if (!existsSync(symbolMapPath)) return [];

  const symbolMap: SymbolMap = JSON.parse(await readFile(symbolMapPath, "utf-8"));

  // Collect source files (non-test, non-doc) with their L0 export signatures
  const sourceFiles = Object.entries(symbolMap)
    .filter(([path]) => !path.endsWith(".md") && !path.endsWith(".mdx") && !path.includes(".spec."))
    .map(([path, entry]) => ({
      path,
      exports: entry.L0.slice(0, 5).join(", ") || "(no exports)",
    }));

  if (sourceFiles.length === 0) return [];

  // Find doc files
  const IGNORE = [`**/${SENSEI_DIR}/**`, "**/.git/**"];
  const docFiles = await fg(["docs/**/*.md", "docs/**/*.mdx", "README.md"], {
    cwd: repoPath,
    ignore: IGNORE,
    absolute: false,
  });

  const results: CoverageEntry[] = [];

  for (const docPath of docFiles) {
    const fullPath = join(repoPath, docPath);
    if (!existsSync(fullPath)) continue;
    const content = await readFile(fullPath, "utf-8");
    const prompt = COVERAGE_PROMPT(docPath, content, sourceFiles);
    const response = await model.generate(prompt);
    const covers = extractJsonArray(response).filter(p => sourceFiles.some(f => f.path === p));
    results.push({ path: docPath, covers });
  }

  return results;
}
