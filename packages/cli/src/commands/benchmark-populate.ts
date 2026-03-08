import { readFileSync, existsSync } from "fs";
import { join } from "path";
import type { SymbolMap } from "@sensei/shared";
import { senseiPath } from "@sensei/shared";

// ── Prompt builder ────────────────────────────────────────────────────────────

export function buildPopulatePrompt(repoPath: string, skillContent: string | null): string {
  // README — first 60 lines
  const readmePath = join(repoPath, "README.md");
  const readmeLines = existsSync(readmePath)
    ? readFileSync(readmePath, "utf-8").split("\n").slice(0, 60).join("\n")
    : "(no README found)";

  // Symbol-map — source files only (no .spec., no .test., no .md)
  const symbolMapPath = senseiPath(repoPath, "symbol-map.json");
  const symbolMap: SymbolMap = existsSync(symbolMapPath)
    ? JSON.parse(readFileSync(symbolMapPath, "utf-8"))
    : {};

  const sourceEntries = Object.entries(symbolMap)
    .filter(([p]) => !p.endsWith(".md") && !p.includes(".spec.") && !p.includes(".test."))
    .map(([p, entry]) => `${p}: ${entry.L0.slice(0, 4).join(", ") || "(no exports)"}`)
    .join("\n");

  // Doc files — exclude plans/ and templates/
  const docEntries = Object.entries(symbolMap)
    .filter(([p]) => p.endsWith(".md") && !p.includes("plans/") && !p.includes("templates/"))
    .map(([p, entry]) => `${p}: ${entry.L0.slice(0, 3).join(", ") || "(no headings)"}`)
    .join("\n");

  // Current llmspec skeleton — strip docs/plans/ and docs/templates/ entries
  const llmspecPath = senseiPath(repoPath, "llmspec.yaml");
  const llmspecRaw = existsSync(llmspecPath)
    ? readFileSync(llmspecPath, "utf-8")
    : "(no llmspec.yaml found)";
  // Remove YAML list items whose path is under docs/plans/ or docs/templates/
  // Each entry looks like:  "  - path: docs/plans/...\n    covers: []"
  const llmspecContent = (() => {
    const lines = llmspecRaw.split("\n");
    const filtered: string[] = [];
    let skip = false;
    for (const line of lines) {
      if (/[ \t]*- path:\s*(docs\/plans\/|docs\/templates\/)/.test(line)) {
        skip = true;
        continue;
      }
      if (skip && /^[ \t]+\S/.test(line)) continue;
      skip = false;
      filtered.push(line);
    }
    return filtered.join("\n");
  })();

  const skillSection = skillContent
    ? `${skillContent}\n\n---\n\n`
    : "";

  return `${skillSection}You are filling in the semantic fields of .sensei/llmspec.yaml.

## README (first 60 lines)
${readmeLines}

## Source files (path → key exports)
${sourceEntries}

## Documentation files (path → headings)
${docEntries}

## Current llmspec.yaml skeleton (fill the TODO fields)
${llmspecContent}

## Instructions

Fill all TODO fields with accurate values based on the repo context above.
For docs[].covers[]:
- Only include files the doc directly documents (explains its API, algorithm, or design)
- Exclude: test/spec files, config files, stub files
- Leave docs/plans/ and docs/templates/ entries with empty covers: []
- Use exact paths from the source files list above

Respond with the complete filled llmspec.yaml content only.
No preamble, no explanation, no markdown code fences.`;
}

// ── Output parser ─────────────────────────────────────────────────────────────

export function parseYamlOutput(text: string): string {
  // Strip markdown code fences (```yaml ... ``` or ``` ... ```)
  const fenceMatch = text.match(/```(?:yaml)?\n([\s\S]*?)\n```/);
  if (fenceMatch) return fenceMatch[1].trim();
  return text.trim();
}

// ── Score parser ──────────────────────────────────────────────────────────────

export function parsePopulateScore(output: string): number {
  const match = output.match(/llmspec coverage score:\s*(\d+)\/100/);
  return match ? parseInt(match[1], 10) : 0;
}
