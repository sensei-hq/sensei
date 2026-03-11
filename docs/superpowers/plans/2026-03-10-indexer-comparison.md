# Indexer Comparison Implementation Plan

> **For agentic workers:** REQUIRED: Use superpowers:subagent-driven-development (if subagents available) or superpowers:executing-plans to implement this plan. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add `sensei benchmark indexer` command that compares cocoindex-code vs. sensei's symbol-map across file coverage, query relevance, and description quality, printing a side-by-side report.

**Architecture:** The comparison has three phases: Collect (query both indexes), Score (compute metrics), Report (print + write markdown). cocoindex is queried via its MCP stdio interface using `@modelcontextprotocol/sdk` (spawning `cocoindex-code serve` as a subprocess); sensei is queried by reading `.sensei/symbol-map.json`. Ground truth for coverage is derived from TypeScript `export` statement parsing.

**Note on spec divergence:** The original spec said "read `target_sqlite.db` directly (no MCP required)". The sqlite-vec extension (`vec0`) required by that database is not loadable via the standard `sqlite3` CLI — it needs the Python cocoindex runtime. Using the MCP stdio interface is the practical equivalent and avoids a Python dependency in the TypeScript code.

**Key insight:** cocoindex stores code *chunks* (semantic search), while sensei stores *named symbols* (structured navigation). The comparison is paradigm-aware: file coverage and query relevance are measured; the spot-check shows raw chunks vs. symbol descriptions for the same files.

**Note on L1 values:** Real sensei `symbol-map.json` L1 entries may be comment-prefixed signatures (e.g. `"// export async function reindexRepo(...)"`) rather than plain descriptions. The reporter strips the leading `// ` when displaying them.

**Tech Stack:** TypeScript, Bun, `@modelcontextprotocol/sdk` (stdio client), vitest

---

## Chunk 1: Types, Ground Truth, and Sensei Adapter

### Task 1: Shared types

**Files:**
- Create: `packages/tools/src/benchmark/indexer-comparison/types.ts`

- [ ] **Step 1: Create types file**

```typescript
/** A file indexed by either indexer */
export interface IndexedFile {
  path: string;
}

/** A code chunk returned by cocoindex search */
export interface CocoChunk {
  filePath: string;
  language: string;
  content: string;
  startLine: number;
  endLine: number;
  score: number;
}

/** A named symbol from sensei's symbol map */
export interface SenseiSymbol {
  name: string;
  path: string;
  L0: string;   // signature
  L1?: string;  // description (may be absent)
}

/** Test query with expected file paths that should appear in results */
export interface TestQuery {
  query: string;
  expectedFiles: string[];  // partial path matches count
}

export interface ComparisonReport {
  cocoFilesIndexed: number;
  senseiFilesIndexed: number;
  groundTruthExports: number;
  cocoCoverage: number;     // 0–1
  senseiCoverage: number;   // 0–1
  queryResults: QueryComparison[];
  spotCheck: SpotCheckRow[];
}

export interface QueryComparison {
  query: string;
  cocoHit: boolean;   // expected file appeared in top-5 results
  senseiHit: boolean;
}

export interface SpotCheckRow {
  filePath: string;
  cocoContent: string | null;   // first chunk content (truncated)
  senseiDescription: string | null; // L1 description or L0 signature
}
```

- [ ] **Step 2: Commit**

```bash
git add packages/tools/src/benchmark/indexer-comparison/types.ts
git commit -m "feat(benchmark): add indexer comparison types"
```

---

### Task 2: Ground truth extractor

**Files:**
- Create: `packages/tools/src/benchmark/indexer-comparison/ground-truth.ts`
- Create: `packages/tools/src/benchmark/indexer-comparison/ground-truth.test.ts`

- [ ] **Step 1: Write failing tests**

```typescript
// packages/tools/src/benchmark/indexer-comparison/ground-truth.test.ts
import { describe, it, expect } from "vitest";
import { extractGroundTruth } from "./ground-truth.js";
import { writeFile, mkdir, rm } from "fs/promises";
import { join } from "path";
import { tmpdir } from "os";

describe("extractGroundTruth", () => {
  let dir: string;

  beforeEach(async () => {
    dir = join(tmpdir(), `gt-test-${Date.now()}`);
    await mkdir(join(dir, "packages", "test-pkg", "src"), { recursive: true });
  });

  afterEach(() => rm(dir, { recursive: true, force: true }));

  it("finds exported functions", async () => {
    await writeFile(join(dir, "packages", "test-pkg", "src", "foo.ts"), `
      export function doThing() {}
      export const helper = () => {};
      function internal() {}
    `);
    const result = await extractGroundTruth(dir);
    expect(result.files).toContain("packages/test-pkg/src/foo.ts");
    expect(result.exportCount).toBe(2);
  });

  it("skips spec files", async () => {
    await writeFile(join(dir, "packages", "test-pkg", "src", "foo.spec.ts"), `export function test() {}`);
    const result = await extractGroundTruth(dir);
    expect(result.exportCount).toBe(0);
  });
});
```

- [ ] **Step 2: Run to confirm failure**

```bash
cd /Users/Jerry/Developer/sensei && bunx vitest run packages/tools/src/benchmark/indexer-comparison/ground-truth.test.ts
```
Expected: FAIL (file not found)

- [ ] **Step 3: Implement**

```typescript
// packages/tools/src/benchmark/indexer-comparison/ground-truth.ts
import fg from "fast-glob";
import { readFile } from "fs/promises";

const EXPORT_RE = /^export\s+(async\s+)?(function|class|const|let|var|type|interface|enum)\s+(\w+)/gm;

export interface GroundTruth {
  files: string[];
  exportCount: number;
}

export async function extractGroundTruth(repoPath: string): Promise<GroundTruth> {
  const tsFiles = await fg(["packages/*/src/**/*.ts", "apps/*/src/**/*.ts"], {
    cwd: repoPath,
    ignore: ["**/*.spec.ts", "**/*.test.ts", "**/node_modules/**", "**/*.d.ts"],
    absolute: false,
  });

  let exportCount = 0;
  for (const file of tsFiles) {
    const content = await readFile(`${repoPath}/${file}`, "utf-8");
    const matches = content.match(EXPORT_RE);
    if (matches) exportCount += matches.length;
  }

  return { files: tsFiles, exportCount };
}
```

- [ ] **Step 4: Run tests to confirm pass**

```bash
bunx vitest run packages/tools/src/benchmark/indexer-comparison/ground-truth.test.ts
```
Expected: PASS

- [ ] **Step 5: Commit**

```bash
git add packages/tools/src/benchmark/indexer-comparison/ground-truth.ts \
        packages/tools/src/benchmark/indexer-comparison/ground-truth.test.ts
git commit -m "feat(benchmark): add ground truth extractor"
```

---

### Task 3: Sensei adapter

**Files:**
- Create: `packages/tools/src/benchmark/indexer-comparison/sensei-adapter.ts`
- Create: `packages/tools/src/benchmark/indexer-comparison/sensei-adapter.test.ts`

- [ ] **Step 1: Write failing tests**

```typescript
// packages/tools/src/benchmark/indexer-comparison/sensei-adapter.test.ts
import { describe, it, expect } from "vitest";
import { loadSenseiIndex } from "./sensei-adapter.js";
import { writeFile, mkdir, rm } from "fs/promises";
import { join } from "path";
import { tmpdir } from "os";

describe("loadSenseiIndex", () => {
  let dir: string;

  beforeEach(async () => {
    dir = join(tmpdir(), `sensei-test-${Date.now()}`);
    await mkdir(join(dir, ".sensei"), { recursive: true });
  });

  afterEach(() => rm(dir, { recursive: true, force: true }));

  it("returns symbols from symbol-map.json", async () => {
    const symbolMap = {
      "packages/tools/src/index.ts": {
        L0: ["export function reindexRepo"],
        L1: ["Rebuilds the index for a repository."],
      }
    };
    await writeFile(join(dir, ".sensei", "symbol-map.json"), JSON.stringify(symbolMap));
    const result = await loadSenseiIndex(dir);
    expect(result.symbols).toHaveLength(1);
    expect(result.symbols[0].name).toBe("reindexRepo");
    expect(result.files).toContain("packages/tools/src/index.ts");
  });

  it("returns empty result when symbol-map missing", async () => {
    const result = await loadSenseiIndex(dir);
    expect(result.symbols).toHaveLength(0);
    expect(result.missing).toBe(true);
  });
});
```

- [ ] **Step 2: Run to confirm failure**

```bash
bunx vitest run packages/tools/src/benchmark/indexer-comparison/sensei-adapter.test.ts
```
Expected: FAIL

- [ ] **Step 3: Implement**

```typescript
// packages/tools/src/benchmark/indexer-comparison/sensei-adapter.ts
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
```

- [ ] **Step 4: Run tests to confirm pass**

```bash
bunx vitest run packages/tools/src/benchmark/indexer-comparison/sensei-adapter.test.ts
```
Expected: PASS

- [ ] **Step 5: Commit**

```bash
git add packages/tools/src/benchmark/indexer-comparison/sensei-adapter.ts \
        packages/tools/src/benchmark/indexer-comparison/sensei-adapter.test.ts
git commit -m "feat(benchmark): add sensei index adapter"
```

---

## Chunk 2: cocoindex Adapter, Scorer, Reporter, CLI

### Task 4: cocoindex MCP adapter

**Files:**
- Create: `packages/tools/src/benchmark/indexer-comparison/cocoindex-adapter.ts`

Note: No unit tests for this adapter — it shells out to the live MCP server. Integration tested via the CLI command.

- [ ] **Step 1: Add MCP SDK to packages/tools**

```bash
cd packages/tools && bun add @modelcontextprotocol/sdk
```

- [ ] **Step 2: Implement adapter**

```typescript
// packages/tools/src/benchmark/indexer-comparison/cocoindex-adapter.ts
import { Client } from "@modelcontextprotocol/sdk/client/index.js";
import { StdioClientTransport } from "@modelcontextprotocol/sdk/client/stdio.js";
import type { CocoChunk } from "./types.js";

export interface CocoIndex {
  files: string[];
  search: (query: string, limit?: number) => Promise<CocoChunk[]>;
  close: () => Promise<void>;
}

export async function connectCocoindex(repoPath: string): Promise<CocoIndex> {
  // Set cwd so cocoindex-code reads .cocoindex_code/ from the right repo root.
  // Do NOT use COCOINDEX_DIR — that env var is not documented/supported.
  const transport = new StdioClientTransport({
    command: "cocoindex-code",
    args: ["serve"],
    env: { ...process.env },
    cwd: repoPath,
  });

  const client = new Client({ name: "sensei-benchmark", version: "1.0.0" });
  await client.connect(transport);

  // Wait for index to be ready (retry up to 30s)
  const deadline = Date.now() + 30_000;
  while (Date.now() < deadline) {
    const result = await client.callTool({
      name: "search",
      arguments: { query: "test", limit: 1, refresh_index: false },
    }) as { content: Array<{ text: string }> };
    const parsed = JSON.parse(result.content[0].text);
    if (parsed.success) break;
    await new Promise(r => setTimeout(r, 1000));
  }

  // Collect all indexed file paths via a broad search
  const allFiles = new Set<string>();
  const probeResult = await client.callTool({
    name: "search",
    arguments: { query: "function", limit: 100, refresh_index: false },
  }) as { content: Array<{ text: string }> };
  const probeData = JSON.parse(probeResult.content[0].text);
  if (probeData.success) {
    for (const r of probeData.results) allFiles.add(r.file_path);
  }

  return {
    files: Array.from(allFiles),
    async search(query: string, limit = 5): Promise<CocoChunk[]> {
      const res = await client.callTool({
        name: "search",
        arguments: { query, limit, refresh_index: false },
      }) as { content: Array<{ text: string }> };
      const data = JSON.parse(res.content[0].text);
      if (!data.success) return [];
      return data.results.map((r: {
        file_path: string; language: string; content: string;
        start_line: number; end_line: number; score: number;
      }) => ({
        filePath: r.file_path, language: r.language, content: r.content,
        startLine: r.start_line, endLine: r.end_line, score: r.score,
      }));
    },
    async close() {
      await client.close();
    },
  };
}
```

- [ ] **Step 3: Smoke test manually**

```bash
cd /Users/Jerry/Developer/sensei
bun -e "
import { connectCocoindex } from './packages/tools/src/benchmark/indexer-comparison/cocoindex-adapter.js';
const idx = await connectCocoindex('.');
console.log('files:', idx.files.length);
const r = await idx.search('reindex repo');
console.log('results:', r.length, r[0]?.filePath);
await idx.close();
"
```
Expected: files > 0, results > 0 with a relevant file path

- [ ] **Step 4: Commit**

```bash
git add packages/tools/src/benchmark/indexer-comparison/cocoindex-adapter.ts
git commit -m "feat(benchmark): add cocoindex MCP adapter"
```

---

### Task 5: Scorer

**Files:**
- Create: `packages/tools/src/benchmark/indexer-comparison/scorer.ts`
- Create: `packages/tools/src/benchmark/indexer-comparison/scorer.test.ts`

- [ ] **Step 1: Write failing tests**

```typescript
// packages/tools/src/benchmark/indexer-comparison/scorer.test.ts
import { describe, it, expect } from "vitest";
import { score } from "./scorer.js";
import type { GroundTruth } from "./ground-truth.js";
import type { CocoIndex } from "./cocoindex-adapter.js";
import type { SenseiIndex } from "./sensei-adapter.js";

const groundTruth: GroundTruth = {
  files: ["packages/tools/src/tools/reindex.ts", "packages/tools/src/tools/drift.ts"],
  exportCount: 10,
};

const cocoIndex = {
  files: ["packages/tools/src/tools/reindex.ts"],
  async search(query: string) {
    if (query.includes("reindex")) return [{ filePath: "packages/tools/src/tools/reindex.ts", language: "typescript", content: "export async function reindexRepo(", startLine: 1, endLine: 5, score: 0.9 }];
    return [];
  },
  async close() {},
} satisfies CocoIndex;

const senseiIndex = {
  symbols: [{ name: "reindexRepo", path: "packages/tools/src/tools/reindex.ts", L0: "export function reindexRepo", L1: "Rebuilds the index." }],
  files: ["packages/tools/src/tools/reindex.ts", "packages/tools/src/tools/drift.ts"],
  missing: false,
} satisfies SenseiIndex;

describe("score", () => {
  it("computes file coverage correctly", async () => {
    const report = await score(groundTruth, cocoIndex, senseiIndex);
    expect(report.cocoFilesIndexed).toBe(1);
    expect(report.senseiFilesIndexed).toBe(2);
    expect(report.cocoCoverage).toBeCloseTo(0.5, 1);  // 1/2 ground truth files
    expect(report.senseiCoverage).toBeCloseTo(1.0, 1); // 2/2 ground truth files
  });

  it("scores query hits correctly", async () => {
    const report = await score(groundTruth, cocoIndex, senseiIndex);
    const reindexQuery = report.queryResults.find(q => q.query.includes("reindex"));
    expect(reindexQuery?.cocoHit).toBe(true);
  });
});
```

- [ ] **Step 2: Run to confirm failure**

```bash
bunx vitest run packages/tools/src/benchmark/indexer-comparison/scorer.test.ts
```
Expected: FAIL

- [ ] **Step 3: Implement**

```typescript
// packages/tools/src/benchmark/indexer-comparison/scorer.ts
import type { GroundTruth } from "./ground-truth.js";
import type { CocoIndex } from "./cocoindex-adapter.js";
import type { SenseiIndex } from "./sensei-adapter.js";
import type { ComparisonReport, QueryComparison, SpotCheckRow } from "./types.js";

const TEST_QUERIES = [
  { query: "reindex repository symbols", expectedFiles: ["reindex.ts"] },
  { query: "check documentation drift", expectedFiles: ["drift.ts"] },
  { query: "load session context", expectedFiles: ["context.ts"] },
  { query: "checkpoint project memory", expectedFiles: ["project-memory.ts"] },
  { query: "list exported symbols", expectedFiles: ["query.ts"] },
];

function fileMatchesAny(filePath: string, patterns: string[]): boolean {
  return patterns.some(p => filePath.includes(p));
}

export async function score(
  groundTruth: GroundTruth,
  cocoIndex: CocoIndex,
  senseiIndex: SenseiIndex
): Promise<ComparisonReport> {
  // Coverage: what fraction of ground-truth TS files does each indexer cover?
  const gtSet = new Set(groundTruth.files);
  const cocoMatched = cocoIndex.files.filter(f => gtSet.has(f)).length;
  const senseiMatched = senseiIndex.files.filter(f => gtSet.has(f)).length;

  // Query relevance
  const queryResults: QueryComparison[] = [];
  for (const { query, expectedFiles } of TEST_QUERIES) {
    const cocoResults = await cocoIndex.search(query, 5);
    const cocoHit = cocoResults.some(r => fileMatchesAny(r.filePath, expectedFiles));

    const senseiHit = senseiIndex.symbols.some(s =>
      fileMatchesAny(s.path, expectedFiles) ||
      (s.L1?.toLowerCase().includes(query.split(" ")[0].toLowerCase()) ?? false)
    );

    queryResults.push({ query, cocoHit, senseiHit });
  }

  // Spot-check: sample up to 15 files present in ground truth
  const sampleFiles = groundTruth.files.slice(0, 15);
  const spotCheck: SpotCheckRow[] = await Promise.all(
    sampleFiles.map(async (filePath): Promise<SpotCheckRow> => {
      const cocoChunks = await cocoIndex.search(`file:${filePath}`, 1);
      const cocoContent = cocoChunks[0]?.content.slice(0, 200) ?? null;

      const senseiSymbol = senseiIndex.symbols.find(s => s.path === filePath);
      const senseiDescription = senseiSymbol?.L1 ?? senseiSymbol?.L0 ?? null;

      return { filePath, cocoContent, senseiDescription };
    })
  );

  return {
    cocoFilesIndexed: cocoIndex.files.length,
    senseiFilesIndexed: senseiIndex.files.length,
    groundTruthExports: groundTruth.exportCount,
    cocoCoverage: gtSet.size > 0 ? cocoMatched / gtSet.size : 0,
    senseiCoverage: gtSet.size > 0 ? senseiMatched / gtSet.size : 0,
    queryResults,
    spotCheck,
  };
}
```

- [ ] **Step 4: Run tests to confirm pass**

```bash
bunx vitest run packages/tools/src/benchmark/indexer-comparison/scorer.test.ts
```
Expected: PASS

- [ ] **Step 5: Commit**

```bash
git add packages/tools/src/benchmark/indexer-comparison/scorer.ts \
        packages/tools/src/benchmark/indexer-comparison/scorer.test.ts
git commit -m "feat(benchmark): add indexer comparison scorer"
```

---

### Task 6: Reporter

**Files:**
- Create: `packages/tools/src/benchmark/indexer-comparison/reporter.ts`

No unit tests — pure formatting logic. Verified visually when running the CLI command.

- [ ] **Step 1: Implement**

```typescript
// packages/tools/src/benchmark/indexer-comparison/reporter.ts
import { writeFile, mkdir } from "fs/promises";
import { join } from "path";
import type { ComparisonReport } from "./types.js";

function pct(n: number): string {
  return `${(n * 100).toFixed(0)}%`;
}

function bar(label: string, a: string, b: string, width = 22): string {
  return `│ ${label.padEnd(width)} │ ${a.padEnd(14)} │ ${b.padEnd(14)} │`;
}

export function printReport(report: ComparisonReport): void {
  const queryHitsCoco = report.queryResults.filter(q => q.cocoHit).length;
  const queryHitsSensei = report.queryResults.filter(q => q.senseiHit).length;

  console.log("\n┌────────────────────────┬────────────────┬────────────────┐");
  console.log("│ Metric                 │ cocoindex      │ sensei         │");
  console.log("├────────────────────────┼────────────────┼────────────────┤");
  console.log(bar("Files indexed", String(report.cocoFilesIndexed), String(report.senseiFilesIndexed)));
  console.log(bar("Coverage (vs TS files)", pct(report.cocoCoverage), pct(report.senseiCoverage)));
  console.log(bar(`Query hits (${report.queryResults.length} queries)`, String(queryHitsCoco), String(queryHitsSensei)));
  console.log("└────────────────────────┴────────────────┴────────────────┘");

  console.log("\nQuery breakdown:");
  for (const q of report.queryResults) {
    const cocoMark = q.cocoHit ? "✓" : "✗";
    const senseiMark = q.senseiHit ? "✓" : "✗";
    console.log(`  [coco:${cocoMark} sensei:${senseiMark}] ${q.query}`);
  }

  if (report.spotCheck.length > 0) {
    console.log("\n── Spot-check (rate descriptions manually 1–5) ──\n");
    for (const row of report.spotCheck) {
      console.log(`File: ${row.filePath}`);
      console.log(`  cocoindex : ${row.cocoContent ?? "(no chunk found)"}`);
      console.log(`  sensei    : ${row.senseiDescription ?? "(not indexed)"}`);
      console.log();
    }
  }
}

export async function writeMarkdownReport(
  report: ComparisonReport,
  repoPath: string
): Promise<string> {
  const date = new Date().toISOString().slice(0, 10);
  const queryHitsCoco = report.queryResults.filter(q => q.cocoHit).length;
  const queryHitsSensei = report.queryResults.filter(q => q.senseiHit).length;

  const lines = [
    `# Indexer Comparison Report — ${date}`,
    "",
    "## Summary",
    "",
    "| Metric | cocoindex | sensei |",
    "|---|---|---|",
    `| Files indexed | ${report.cocoFilesIndexed} | ${report.senseiFilesIndexed} |`,
    `| Coverage (vs TS files) | ${pct(report.cocoCoverage)} | ${pct(report.senseiCoverage)} |`,
    `| Query hits (${report.queryResults.length} queries) | ${queryHitsCoco} | ${queryHitsSensei} |`,
    "",
    "## Query Breakdown",
    "",
    ...report.queryResults.map(q =>
      `- \`${q.query}\`: coco ${q.cocoHit ? "✓" : "✗"} · sensei ${q.senseiHit ? "✓" : "✗"}`
    ),
    "",
    "## Spot-check",
    "",
    ...report.spotCheck.flatMap(row => [
      `**${row.filePath}**`,
      `- cocoindex: \`${row.cocoContent?.slice(0, 150) ?? "(no chunk)"}\``,
      `- sensei: \`${row.senseiDescription ?? "(not indexed)"}\``,
      "",
    ]),
    "## Decision",
    "",
    "> Fill in after manual review.",
    "",
  ];

  const outDir = join(repoPath, "results");
  await mkdir(outDir, { recursive: true });
  const outPath = join(outDir, `indexer-comparison-${date}.md`);
  await writeFile(outPath, lines.join("\n"));
  return outPath;
}
```

- [ ] **Step 2: Commit**

```bash
git add packages/tools/src/benchmark/indexer-comparison/reporter.ts
git commit -m "feat(benchmark): add indexer comparison reporter"
```

---

### Task 7: CLI command

**Files:**
- Create: `packages/cli/src/commands/benchmark-indexer.ts`
- Modify: `packages/cli/src/cli.ts`

- [ ] **Step 1: Wire into cli.ts**

In `packages/cli/src/cli.ts`, add to the `benchmark` case block (after the `populate` branch, before the closing `else`):

```typescript
} else if (subCmd === "indexer") {
  const { benchmarkIndexer } = await import("./commands/benchmark-indexer.js");
  await benchmarkIndexer(repoRoot);
```

Also add to the HELP string under `benchmark coverage:`:
```
benchmark indexer:
  Compare cocoindex-code vs sensei's symbol indexer.
  Measures file coverage, query relevance, and prints a spot-check for manual review.
  Requires: cocoindex-code installed (pipx install cocoindex-code) and indexed.
```

- [ ] **Step 3: Export benchmark modules from @sensei/tools**

`packages/tools/package.json` only exports `"."`. Subpath imports like `@sensei/tools/benchmark/...` won't resolve. Fix by re-exporting from `packages/tools/src/index.ts`:

```typescript
// Add to bottom of packages/tools/src/index.ts

// Benchmark: indexer comparison
export { extractGroundTruth } from "./benchmark/indexer-comparison/ground-truth.js";
export type { GroundTruth } from "./benchmark/indexer-comparison/ground-truth.js";
export { loadSenseiIndex } from "./benchmark/indexer-comparison/sensei-adapter.js";
export type { SenseiIndex } from "./benchmark/indexer-comparison/sensei-adapter.js";
export { connectCocoindex } from "./benchmark/indexer-comparison/cocoindex-adapter.js";
export type { CocoIndex } from "./benchmark/indexer-comparison/cocoindex-adapter.js";
export { score } from "./benchmark/indexer-comparison/scorer.js";
export { printReport, writeMarkdownReport } from "./benchmark/indexer-comparison/reporter.js";
```

And update `benchmark-indexer.ts` to import from `@sensei/tools` (single top-level import):

```typescript
// packages/cli/src/commands/benchmark-indexer.ts
import {
  reindexRepo,
  extractGroundTruth,
  loadSenseiIndex,
  connectCocoindex,
  score,
  printReport,
  writeMarkdownReport,
} from "@sensei/tools";
import { existsSync } from "fs";
import { join } from "path";

export async function benchmarkIndexer(repoPath: string): Promise<void> {
  const symbolMapPath = join(repoPath, ".sensei", "symbol-map.json");
  if (!existsSync(symbolMapPath)) {
    console.log("sensei: symbol-map.json not found, running indexer...");
    await reindexRepo(repoPath, { force: false });
  }

  console.log("Extracting ground truth from TypeScript exports...");
  const groundTruth = await extractGroundTruth(repoPath);
  console.log(`  Found ${groundTruth.files.length} TS source files, ${groundTruth.exportCount} exports`);

  console.log("Loading sensei index...");
  const senseiIndex = await loadSenseiIndex(repoPath);
  if (senseiIndex.missing) {
    console.error("sensei: symbol-map.json still missing after reindex. Something is wrong.");
    process.exit(1);
  }
  console.log(`  ${senseiIndex.symbols.length} symbols across ${senseiIndex.files.length} files`);

  console.log("Connecting to cocoindex-code MCP server (waiting for index)...");
  let cocoIndex;
  try {
    cocoIndex = await connectCocoindex(repoPath);
  } catch (err: unknown) {
    const msg = err instanceof Error ? err.message : String(err);
    console.error(`sensei: Failed to connect to cocoindex-code: ${msg}`);
    console.error("Make sure cocoindex-code is installed: pipx install cocoindex-code");
    console.error("And the index is built: cd <repo> && cocoindex-code index");
    process.exit(1);
  }
  console.log(`  ${cocoIndex.files.length} files indexed`);

  console.log("Scoring...");
  const report = await score(groundTruth, cocoIndex, senseiIndex);
  await cocoIndex.close();

  printReport(report);
  const outPath = await writeMarkdownReport(report, repoPath);
  console.log(`\nReport written to: ${outPath}`);
}
```

Build to confirm no errors:

```bash
cd /Users/Jerry/Developer/sensei && bun run build
```
Expected: no errors

- [ ] **Step 4: Run the full command**

```bash
cd /Users/Jerry/Developer/sensei && sensei benchmark indexer
```

Expected output:
```
Extracting ground truth from TypeScript exports...
  Found N TS source files, M exports
Loading sensei index...
  X symbols across Y files
Connecting to cocoindex-code MCP server (waiting for index)...
  Z files indexed

┌────────────────────────┬────────────────┬────────────────┐
│ Metric                 │ cocoindex      │ sensei         │
...
Report written to: results/indexer-comparison-2026-03-10.md
```

- [ ] **Step 5: Commit**

```bash
git add packages/cli/src/commands/benchmark-indexer.ts packages/cli/src/cli.ts
git commit -m "feat(cli): add benchmark indexer command"
```

---

## Post-run: Decision

After running the comparison, fill in the **Decision** section of `results/indexer-comparison-YYYY-MM-DD.md`:

- If cocoindex coverage > 2× sensei AND spot-check descriptions are richer → replace sensei's indexer with cocoindex as the backend
- If roughly equivalent or cocoindex is harder to set up → invest in improving sensei's extractor
- If cocoindex wins for semantic search but sensei wins for structured navigation → keep both; cocoindex for `find_pattern`/`query_index` MCP tools, sensei for `get_file_context`/`list_exports`
