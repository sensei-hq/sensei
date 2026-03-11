# Indexer Comparison Report — 2026-03-11

## Summary

| Metric | cocoindex | sensei |
|---|---|---|
| Files indexed | 42 | 81 |
| Coverage (vs TS files) | 44% | 63% |
| Query hits (5 queries) | 0 | 5 |

## Query Breakdown

- `reindex repository symbols`: coco ✗ · sensei ✓
- `check documentation drift`: coco ✗ · sensei ✓
- `load session context`: coco ✗ · sensei ✓
- `checkpoint project memory`: coco ✗ · sensei ✓
- `list exported symbols`: coco ✗ · sensei ✓

## Spot-check

**packages/mcp/src/index.ts**
- cocoindex: `# Index Summary Output

After indexing, the developer sees what changed.

```gherkin
Feature: Index Summary

  Scenario: Summary shown after increment`
- sensei: `(not indexed)`

**packages/server/src/serve.ts**
- cocoindex: `// Thin re-export — implementation lives in @sensei/server
export { serve } from "@sensei/server";
`
- sensei: `export interface ServeOptions`

**packages/server/src/index.ts**
- cocoindex: `# Index Summary Output

After indexing, the developer sees what changed.

```gherkin
Feature: Index Summary

  Scenario: Summary shown after increment`
- sensei: `(not indexed)`

**packages/tools/src/index-reader.ts**
- cocoindex: ` Architecture

Three phases: Collect → Score → Report.

### Components

All under `packages/tools/src/benchmark/indexer-comparison/`:

- **`cocoindex-`
- sensei: `export async function readLlmSpec(repoPath: string): Promise<LlmSpec>`

**packages/tools/src/index.ts**
- cocoindex: `# Index Summary Output

After indexing, the developer sees what changed.

```gherkin
Feature: Index Summary

  Scenario: Summary shown after increment`
- sensei: `(not indexed)`

**packages/shared/src/types.ts**
- cocoindex: `del / inference types ─────────────────────────────────────────────────

export interface AnalyzedSymbol {
  name: string;
  kind: "function" | "class`
- sensei: `export interface LlmSpec`

**packages/shared/src/constants.ts**
- cocoindex: `export * from "./types.js";
export * from "./constants.js";`
- sensei: `The folder where sensei writes all generated artifacts.
// export const SENSEI_DIR`

**packages/shared/src/index.ts**
- cocoindex: `# Index Summary Output

After indexing, the developer sees what changed.

```gherkin
Feature: Index Summary

  Scenario: Summary shown after increment`
- sensei: `(not indexed)`

**packages/cli/src/cli.ts**
- cocoindex: `# CLI

## Overview

`sensei` is a TypeScript CLI installed globally via `bun add -g sensei` or `npx sensei`. It shares the same codebase as the MCP se`
- sensei: `async function main`

**packages/cli/src/names.ts**
- cocoindex: ``
- sensei: `export function generateRunName(): string`

**packages/cli/src/claude.ts**
- cocoindex: ``
- sensei: `export interface ClaudeUsage`

**packages/cli/src/git.ts**
- cocoindex: `
results/
  2026-03-06-benchmark.json       Raw results (gitignored)
  2026-03-06-comparison.md        Human-readable summary (committed)
  .gitignore`
- sensei: `Find the repo root starting from `cwd`. Tries `git rev-parse --show-toplevel`, then walks up looking for package.json. R
// export function findRepoRoot(cwd: string): string`

**packages/server/src/__stubs__/bun-sqlite.ts**
- cocoindex: `/**
 * Stub for bun:sqlite used in Vitest (Node.js) test runs.
 * Provides the minimal Database interface that serve.ts uses:
 *   new Database(path)
`
- sensei: `Stub for bun:sqlite used in Vitest (Node.js) test runs. Provides the minimal Database interface that serve.ts uses: new 
// export class Database`

**packages/server/src/__stubs__/bun-globals.ts**
- cocoindex: `/**
 * Vitest setup file: polyfill Bun globals for Node.js test runs.
 *
 * Implements the subset of the Bun API used by serve.ts:
 *   Bun.serve({ po`
- sensei: `function bunServe`

**packages/server/src/model/system-check.ts**
- cocoindex: `tests to verify pass**

```bash
bunx vitest run src/model/system-check.spec.ts
```

Expected: 3 passing

**Step 5: Commit**

```bash
cd /Users/Jerry/D`
- sensei: `export const OLLAMA_MODEL`

## Decision

> Fill in after manual review.
