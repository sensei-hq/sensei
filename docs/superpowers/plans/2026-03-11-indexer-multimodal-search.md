# Indexer Multi-Modal Search Implementation Plan

> **For agentic workers:** REQUIRED: Use superpowers:subagent-driven-development (if subagents available) or superpowers:executing-plans to implement this plan. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add multi-modal search (symbol + BM25 + semantic) to sensei, unified via RRF, with a `sensei watch` file watcher.

**Architecture:** Three search layers — exact/prefix symbol match on symbol-map.json, BM25 full-text over per-chunk term frequencies stored in chunks.json, and cosine similarity over 384-dim Transformer.js embeddings in embeddings.json — merged via Reciprocal Rank Fusion. `buildChunksAndEmbeddings()` is called from `reindexRepo()` after the symbol-map write. The MCP `search` tool exposes all three layers. `sensei watch` uses chokidar with a 500ms debounce for incremental reindexing.

**Tech Stack:** TypeScript, Bun, Vitest, `@xenova/transformers` (all-MiniLM-L6-v2, ONNX), `chokidar`, fast-glob (already present)

**Spec:** `docs/superpowers/specs/2026-03-11-indexer-multimodal-search-design.md`

---

## Chunk 1: Core data layer (chunker, BM25, embedder)

### Task 1: Chunker

**Files:**
- Create: `packages/tools/src/tools/chunker.ts`
- Create: `packages/tools/src/tools/chunker.spec.ts`

**Chunk schema** (stored in `.sensei/chunks.json`):
```typescript
interface Chunk {
  file: string
  type: 'symbol' | 'doc'
  text: string
  contentHash: string
  tf: Record<string, number>
}

interface ChunksFile {
  version: 1
  corpusSize: number
  avgChunkLength: number
  chunks: Record<string, Chunk>  // id → Chunk
}
```

Chunk IDs:
- Code: `"src/auth.ts:login"` (file:symbolName — use L0 line up to first `(` or ` ` for the name part)
- Doc: `"docs/design/05-indexing.md#symbol-map"` (file#headingText with spaces→hyphens, lowercase)

Chunk text:
- Code symbol: L0 signature + `\n` + L1 description (if L1 description differs from just `// ${L0}`). Symbols without L1 description use L0 only. Max 300 chars.
- Doc section: heading text + `\n` + first 400 chars of section body (text only, no markdown syntax).

`tf` is term → raw count. Tokenize: `text.toLowerCase().split(/[^a-z0-9]+/).filter(Boolean)`.

`contentHash`: `createHash('sha256').update(text).digest('hex').slice(0, 16)`

`buildChunksAndEmbeddings` signature (called from reindex.ts):
```typescript
export async function buildChunksAndEmbeddings(
  repoPath: string,
  symbolMap: SymbolMap,
  docFiles: string[],
  options?: { force?: boolean }
): Promise<void>
```

This function:
1. Reads existing `.sensei/chunks.json` and `.sensei/embeddings.json` (or starts empty)
2. Calls `extractChunks(symbolMap, docFiles, repoPath)` to get new chunk set
3. For each new/changed chunk (contentHash differs): calls `embed(text)`, updates embeddings
4. For unchanged chunks: keeps existing vector, skips embed()
5. Removes chunks for files no longer in symbolMap or docFiles
6. Writes updated chunks.json and embeddings.json

`extractChunks` is an async function, exported for testing:
```typescript
export async function extractChunks(
  symbolMap: SymbolMap,
  docFiles: string[],
  repoPath: string
): Promise<Record<string, { file: string; type: "symbol" | "doc"; text: string }>>
```
Returns raw `{ file, type, text }` per chunk. `contentHash` and `tf` are added by `buildChunksAndEmbeddings`.

- [ ] **Step 1: Write the failing tests**

```typescript
// packages/tools/src/tools/chunker.spec.ts
import { describe, it, expect, vi, beforeEach, afterEach } from "vitest";
import { mkdirSync, writeFileSync, rmSync, existsSync } from "fs";
import { join } from "path";
import { extractChunks, buildChunksAndEmbeddings } from "./chunker.js";
import { senseiPath } from "@sensei/shared";
import type { SymbolMap } from "@sensei/shared";

// vi.hoisted() ensures mockEmbed is initialized before the vi.mock factory executes
// (Vitest hoists vi.mock calls above all imports; any variable used in the factory
//  must also be hoisted or it will be undefined when the factory runs)
const mockEmbed = vi.hoisted(() => vi.fn().mockResolvedValue(new Array(384).fill(0.1)));
vi.mock("./embedder.js", () => ({
  embed: mockEmbed,
  isAvailable: vi.fn().mockResolvedValue(true),
  ensureReady: vi.fn().mockResolvedValue(undefined),
}));

const TMP = "/tmp/sensei-test-chunker";

beforeEach(() => {
  mkdirSync(join(TMP, ".sensei"), { recursive: true });
  mkdirSync(join(TMP, "src"), { recursive: true });
  mkdirSync(join(TMP, "docs"), { recursive: true });
  mockEmbed.mockClear();
});
afterEach(() => rmSync(TMP, { recursive: true, force: true }));

describe("extractChunks", () => {
  it("produces one chunk per symbol from symbol map", async () => {
    const symbolMap: SymbolMap = {
      "src/auth.ts": {
        L0: ["login(email: string, password: string): Promise<User | null>"],
        L1: ["// Authenticate user and return session token or null on failure\n// login(email: string, password: string): Promise<User | null>"],
        L2: [],
      }
    };
    const chunks = await extractChunks(symbolMap, [], TMP);
    const id = "src/auth.ts:login";
    expect(chunks[id]).toBeDefined();
    expect(chunks[id].file).toBe("src/auth.ts");
    expect(chunks[id].type).toBe("symbol");
    expect(chunks[id].text).toContain("login");
  });

  it("produces one chunk per H2/H3 section for markdown", async () => {
    writeFileSync(join(TMP, "docs/design.md"), [
      "# Design",
      "",
      "## Symbol Map",
      "",
      "Extracted exports stored at L0 and L1 per file.",
      "",
      "### Storage Format",
      "",
      "Stored as JSON.",
    ].join("\n"));
    const chunks = await extractChunks({}, ["docs/design.md"], TMP);
    expect(chunks["docs/design.md#symbol-map"]).toBeDefined();
    expect(chunks["docs/design.md#storage-format"]).toBeDefined();
    expect(chunks["docs/design.md#symbol-map"].type).toBe("doc");
    expect(chunks["docs/design.md#symbol-map"].text).toContain("Symbol Map");
  });

  it("returns empty object for empty file with no symbols", async () => {
    const symbolMap: SymbolMap = { "src/empty.ts": { L0: [], L1: [], L2: [] } };
    const chunks = await extractChunks(symbolMap, [], TMP);
    const symbolChunks = Object.values(chunks).filter(c => c.file === "src/empty.ts");
    expect(symbolChunks).toHaveLength(0);
  });
});

describe("buildChunksAndEmbeddings", () => {
  it("writes chunks.json and embeddings.json", async () => {
    const symbolMap: SymbolMap = {
      "src/auth.ts": {
        L0: ["login(email: string): Promise<User>"],
        L1: ["// login(email: string): Promise<User>"],
        L2: [],
      }
    };
    await buildChunksAndEmbeddings(TMP, symbolMap, [], {});
    expect(existsSync(senseiPath(TMP, "chunks.json"))).toBe(true);
    expect(existsSync(senseiPath(TMP, "embeddings.json"))).toBe(true);
  });

  it("does not call embed() for unchanged chunks (contentHash match)", async () => {
    const symbolMap: SymbolMap = {
      "src/auth.ts": {
        L0: ["login(email: string): Promise<User>"],
        L1: ["// login(email: string): Promise<User>"],
        L2: [],
      }
    };
    // First run — should call embed() for the new chunk
    await buildChunksAndEmbeddings(TMP, symbolMap, [], {});
    expect(mockEmbed.mock.calls.length).toBeGreaterThan(0);

    // Second run with same symbolMap — contentHash unchanged, must NOT re-embed
    mockEmbed.mockClear();
    await buildChunksAndEmbeddings(TMP, symbolMap, [], {});
    expect(mockEmbed.mock.calls.length).toBe(0);
  });
});
```

- [ ] **Step 2: Run tests to verify they fail**

```bash
cd /path/to/repo && bun test packages/tools/src/tools/chunker.spec.ts
```
Expected: FAIL — "Cannot find module './chunker.js'"

- [ ] **Step 3: Implement `chunker.ts`**

```typescript
// packages/tools/src/tools/chunker.ts
import { readFile, writeFile } from "fs/promises";
import { existsSync } from "fs";
import { join } from "path";
import { createHash } from "crypto";
import type { SymbolMap } from "@sensei/shared";
import { senseiPath } from "@sensei/shared";
import { embed, ensureReady } from "./embedder.js";

interface Chunk {
  file: string;
  type: "symbol" | "doc";
  text: string;
  contentHash: string;
  tf: Record<string, number>;
}

interface ChunksFile {
  version: 1;
  corpusSize: number;
  avgChunkLength: number;
  chunks: Record<string, Chunk>;
}

interface EmbeddingsFile {
  version: 1;
  model: string;
  dimensions: number;
  vectors: Record<string, number[]>;
}

function tokenize(text: string): string[] {
  return text.toLowerCase().split(/[^a-z0-9]+/).filter(Boolean);
}

function computeTf(text: string): Record<string, number> {
  const tf: Record<string, number> = {};
  for (const term of tokenize(text)) {
    tf[term] = (tf[term] ?? 0) + 1;
  }
  return tf;
}

function contentHash(text: string): string {
  return createHash("sha256").update(text).digest("hex").slice(0, 16);
}

function symbolNameFromL0(l0: string): string {
  // Extract name: everything before first `(`, `<`, ` `, or `:`
  return l0.replace(/^(?:export\s+)?(?:async\s+)?(?:function|class|const|type|interface|enum)\s+/, "")
           .split(/[(<: ]/)[0]
           .trim();
}

function chunkTextForSymbol(l0: string, l1: string): string {
  // l1 typically looks like "// description\n// signature" or just "// signature"
  const lines = l1.split("\n").map(l => l.replace(/^\/\/\s?/, "").trim()).filter(Boolean);
  const hasDescription = lines.length >= 2 || (lines.length === 1 && lines[0] !== l0.trim());
  const text = hasDescription ? `${l0}\n${lines.join(" ")}` : l0;
  return text.slice(0, 300);
}

function sectionId(file: string, heading: string): string {
  const slug = heading.replace(/^#{2,3}\s+/, "").toLowerCase().replace(/[^a-z0-9]+/g, "-").replace(/^-|-$/g, "");
  return `${file}#${slug}`;
}

export async function extractChunks(
  symbolMap: SymbolMap,
  docFiles: string[],
  repoPath: string
): Promise<Record<string, { file: string; type: "symbol" | "doc"; text: string }>> {
  const result: Record<string, { file: string; type: "symbol" | "doc"; text: string }> = {};

  // Code chunks — one per symbol from symbol-map
  for (const [file, symbols] of Object.entries(symbolMap)) {
    const l0s = symbols.L0 ?? [];
    const l1s = symbols.L1 ?? [];
    for (let i = 0; i < l0s.length; i++) {
      const l0 = l0s[i];
      const l1 = l1s[i] ?? `// ${l0}`;
      const name = symbolNameFromL0(l0);
      if (!name) continue;
      const id = `${file}:${name}`;
      const text = chunkTextForSymbol(l0, l1);
      result[id] = { file, type: "symbol", text };
    }
  }

  // Doc chunks — one per H2/H3 section
  for (const docFile of docFiles) {
    if (!docFile.endsWith(".md") && !docFile.endsWith(".mdx")) continue;
    const fullPath = join(repoPath, docFile);
    if (!existsSync(fullPath)) continue;
    try {
      const content = await readFile(fullPath, "utf-8");
      const lines = content.split("\n");
      let currentHeading: string | null = null;
      let bodyLines: string[] = [];

      const flush = () => {
        if (!currentHeading) return;
        const headingText = currentHeading.replace(/^#{2,3}\s+/, "");
        const bodyText = bodyLines
          .join(" ")
          .replace(/[#*`_\[\]]/g, "")
          .replace(/\s+/g, " ")
          .trim()
          .slice(0, 400);
        const text = `${headingText}\n${bodyText}`.trim();
        if (text) {
          const id = sectionId(docFile, currentHeading);
          result[id] = { file: docFile, type: "doc", text };
        }
      };

      for (const line of lines) {
        if (/^#{2,3} /.test(line)) {
          flush();
          currentHeading = line;
          bodyLines = [];
        } else if (currentHeading) {
          bodyLines.push(line);
        }
      }
      flush();
    } catch { /* skip unreadable */ }
  }

  return result;
}

export async function buildChunksAndEmbeddings(
  repoPath: string,
  symbolMap: SymbolMap,
  docFiles: string[],
  options?: { force?: boolean }
): Promise<void> {
  const chunksPath = senseiPath(repoPath, "chunks.json");
  const embeddingsPath = senseiPath(repoPath, "embeddings.json");

  // Load existing data
  let existingChunks: ChunksFile["chunks"] = {};
  let existingVectors: Record<string, number[]> = {};

  if (!options?.force) {
    if (existsSync(chunksPath)) {
      try {
        const data = JSON.parse(await readFile(chunksPath, "utf-8")) as ChunksFile;
        existingChunks = data.chunks ?? {};
      } catch { /* start fresh */ }
    }
    if (existsSync(embeddingsPath)) {
      try {
        const data = JSON.parse(await readFile(embeddingsPath, "utf-8")) as EmbeddingsFile;
        existingVectors = data.vectors ?? {};
      } catch { /* start fresh */ }
    }
  }

  // Ensure model is ready before the embedding loop — downloads if not cached
  let embeddingAvailable = true;
  try {
    await ensureReady();
  } catch {
    embeddingAvailable = false;
    console.warn("Semantic search unavailable — run sensei index to generate embeddings");
  }

  const rawChunks = await extractChunks(symbolMap, docFiles, repoPath);
  const newChunks: ChunksFile["chunks"] = {};
  const newVectors: Record<string, number[]> = {};

  for (const [id, raw] of Object.entries(rawChunks)) {
    const hash = contentHash(raw.text);
    const tf = computeTf(raw.text);
    newChunks[id] = { file: raw.file, type: raw.type, text: raw.text, contentHash: hash, tf };

    // Re-use existing vector if text unchanged
    if (existingChunks[id]?.contentHash === hash && existingVectors[id]) {
      newVectors[id] = existingVectors[id];
    } else if (embeddingAvailable) {
      try {
        newVectors[id] = await embed(raw.text);
      } catch {
        // Embedding failed for this chunk — omit vector, continue
      }
    }
  }

  const tokenCounts = Object.values(newChunks).map(c => tokenize(c.text).length);
  const corpusSize = tokenCounts.length;
  const avgChunkLength = corpusSize > 0
    ? Math.round(tokenCounts.reduce((a, b) => a + b, 0) / corpusSize)
    : 0;

  const chunksFile: ChunksFile = {
    version: 1,
    corpusSize,
    avgChunkLength,
    chunks: newChunks,
  };

  const embeddingsFile: EmbeddingsFile = {
    version: 1,
    model: "Xenova/all-MiniLM-L6-v2",
    dimensions: 384,
    vectors: newVectors,
  };

  await Promise.all([
    writeFile(chunksPath, JSON.stringify(chunksFile, null, 2)),
    writeFile(embeddingsPath, JSON.stringify(embeddingsFile, null, 2)),
  ]);
}
```

- [ ] **Step 4: Run tests to verify they pass**

```bash
bun test packages/tools/src/tools/chunker.spec.ts
```
Expected: all tests pass

- [ ] **Step 5: Commit**

```bash
git add packages/tools/src/tools/chunker.ts packages/tools/src/tools/chunker.spec.ts
git commit -m "feat(tools): add chunker — extract symbol and doc chunks for BM25 + semantic search"
```

---

### Task 2: BM25 Scorer

**Files:**
- Create: `packages/tools/src/tools/bm25.ts`
- Create: `packages/tools/src/tools/bm25.spec.ts`

BM25 is a pure stateless function. Given a corpus (chunks.json data) and a query, returns ranked results.

Constants: `k1 = 1.5`, `b = 0.75`

IDF formula:
```
IDF(term) = log( (N - df(term) + 0.5) / (df(term) + 0.5) + 1 )
```
where `N = corpusSize`, `df(term)` = number of chunks where `tf[term] > 0` (derived at query time by scanning chunks).

BM25 score per chunk:
```
score(chunk, query) = Σ_term  IDF(term) * (tf[term] * (k1 + 1)) / (tf[term] + k1 * (1 - b + b * |chunk| / avgLen))
```
where `|chunk|` = total token count of chunk = `sum(tf values)`.

Exported interface:
```typescript
export function scoreBM25(
  query: string,
  chunks: Record<string, { tf: Record<string, number> }>,
  corpusSize: number,
  avgChunkLength: number
): Array<{ id: string; score: number }>
// Returns sorted descending, score > 0 only
```

- [ ] **Step 1: Write the failing tests**

```typescript
// packages/tools/src/tools/bm25.spec.ts
import { describe, it, expect } from "vitest";
import { scoreBM25 } from "./bm25.js";

const authChunk = { tf: { login: 2, email: 1, authenticate: 1, user: 2 } };
const homeChunk = { tf: { home: 3, page: 2, render: 1 } };
const corpus = { "src/auth.ts:login": authChunk, "src/home.ts:render": homeChunk };

describe("scoreBM25", () => {
  it("returns auth chunk first for 'login' query", () => {
    const results = scoreBM25("login", corpus, 2, 5);
    expect(results[0].id).toBe("src/auth.ts:login");
    expect(results[0].score).toBeGreaterThan(0);
  });

  it("returns only positive-score results", () => {
    const results = scoreBM25("unrelated", corpus, 2, 5);
    expect(results.every(r => r.score > 0)).toBe(true);
    expect(results).toHaveLength(0);
  });

  it("term in all chunks scores near zero for IDF", () => {
    // With N=20 chunks all containing 'common': df=20
    // IDF = log((20-20+0.5)/(20+0.5)+1) = log(0.0244+1) ≈ 0.0241
    // score ≈ 0.0241 * (1 * 2.5) / (1 + 1.5) ≈ 0.024 → well below 0.1
    const ubiquitous = Object.fromEntries(
      Array.from({ length: 20 }, (_, i) => [`chunk-${i}`, { tf: { common: 1 } }])
    );
    const results = scoreBM25("common", ubiquitous, 20, 1);
    results.forEach(r => expect(r.score).toBeLessThan(0.1));
  });

  it("longer documents are penalised (b=0.75 effect)", () => {
    // Same term frequency, one chunk is longer
    const short = { tf: { auth: 1, x: 1 } };            // length 2
    const long  = { tf: { auth: 1, a: 1, b: 1, c: 1, d: 1, e: 1 } }; // length 6
    const mixed = { "short": short, "long": long };
    const results = scoreBM25("auth", mixed, 2, 4);
    const shortScore = results.find(r => r.id === "short")?.score ?? 0;
    const longScore  = results.find(r => r.id === "long")?.score ?? 0;
    expect(shortScore).toBeGreaterThan(longScore);
  });

  it("returns results sorted descending by score", () => {
    const results = scoreBM25("authenticate user", corpus, 2, 5);
    for (let i = 1; i < results.length; i++) {
      expect(results[i - 1].score).toBeGreaterThanOrEqual(results[i].score);
    }
  });
});
```

- [ ] **Step 2: Run tests to verify they fail**

```bash
bun test packages/tools/src/tools/bm25.spec.ts
```
Expected: FAIL — "Cannot find module './bm25.js'"

- [ ] **Step 3: Implement `bm25.ts`**

```typescript
// packages/tools/src/tools/bm25.ts
const K1 = 1.5;
const B = 0.75;

function tokenize(text: string): string[] {
  return text.toLowerCase().split(/[^a-z0-9]+/).filter(Boolean);
}

export function scoreBM25(
  query: string,
  chunks: Record<string, { tf: Record<string, number> }>,
  corpusSize: number,
  avgChunkLength: number
): Array<{ id: string; score: number }> {
  const queryTerms = tokenize(query);
  if (queryTerms.length === 0) return [];

  // Compute df per query term
  const df: Record<string, number> = {};
  for (const term of queryTerms) {
    df[term] = 0;
    for (const chunk of Object.values(chunks)) {
      if ((chunk.tf[term] ?? 0) > 0) df[term]++;
    }
  }

  const results: Array<{ id: string; score: number }> = [];

  for (const [id, chunk] of Object.entries(chunks)) {
    const chunkLen = Object.values(chunk.tf).reduce((a, b) => a + b, 0);
    let score = 0;

    for (const term of queryTerms) {
      const termFreq = chunk.tf[term] ?? 0;
      if (termFreq === 0) continue;

      const n = corpusSize;
      const d = df[term] ?? 0;
      const idf = Math.log((n - d + 0.5) / (d + 0.5) + 1);
      const norm = K1 * (1 - B + B * chunkLen / (avgChunkLength || 1));
      score += idf * (termFreq * (K1 + 1)) / (termFreq + norm);
    }

    if (score > 0) results.push({ id, score });
  }

  return results.sort((a, b) => b.score - a.score);
}
```

- [ ] **Step 4: Run tests to verify they pass**

```bash
bun test packages/tools/src/tools/bm25.spec.ts
```
Expected: all tests pass

- [ ] **Step 5: Commit**

```bash
git add packages/tools/src/tools/bm25.ts packages/tools/src/tools/bm25.spec.ts
git commit -m "feat(tools): add BM25 scorer — k1=1.5, b=0.75, IDF computed at query time"
```

---

### Task 3: Embedder

**Files:**
- Create: `packages/tools/src/tools/embedder.ts`
- Create: `packages/tools/src/tools/embedder.spec.ts`
- Modify: `packages/tools/package.json` — add `@xenova/transformers` dependency

The embedder wraps `@xenova/transformers`. The model is lazily loaded on first use. `embed()` returns a 384-dim vector. `isAvailable()` checks if the model cache exists at `~/.cache/xenova/`. `ensureReady()` downloads the model if not cached (called at index time from `buildChunksAndEmbeddings`).

**Note:** Because `@xenova/transformers` downloads a real 80MB model, the tests for `embed()` and `isAvailable()` must mock the transformer module entirely. The spec verifies shape and determinism via mocks only.

- [ ] **Step 1: Add `@xenova/transformers` to tools package.json**

In `packages/tools/package.json`, add to `dependencies`:
```json
"@xenova/transformers": "^2.17.2"
```

Then run:
```bash
bun install
```

- [ ] **Step 2: Write the failing tests**

```typescript
// packages/tools/src/tools/embedder.spec.ts
import { describe, it, expect, vi, beforeEach, afterEach } from "vitest";

// Mock @xenova/transformers before importing embedder
vi.mock("@xenova/transformers", () => ({
  pipeline: vi.fn().mockResolvedValue(
    vi.fn().mockResolvedValue({
      data: new Float32Array(384).fill(0.1),
    })
  ),
  env: { cacheDir: "/tmp/xenova-test-cache", localModelPath: "" },
}));

import { embed, isAvailable, ensureReady } from "./embedder.js";

describe("embed", () => {
  it("returns array of length 384", async () => {
    const result = await embed("hello world");
    expect(result).toHaveLength(384);
    expect(typeof result[0]).toBe("number");
  });

  it("same text input produces same vector (deterministic)", async () => {
    const a = await embed("authenticate user");
    const b = await embed("authenticate user");
    expect(a).toEqual(b);
  });

  it("reuses pipeline singleton — pipeline() called only once across multiple embed() calls", async () => {
    const { pipeline } = await import("@xenova/transformers");
    const pipelineMock = vi.mocked(pipeline);
    pipelineMock.mockClear();
    await embed("first call");
    await embed("second call");
    // pipeline() should have been called at most once (lazy singleton)
    expect(pipelineMock.mock.calls.length).toBeLessThanOrEqual(1);
  });
});

describe("isAvailable", () => {
  it("returns false when cache directory absent", async () => {
    // The mock env.cacheDir points to /tmp/xenova-test-cache which doesn't exist
    const result = await isAvailable();
    expect(result).toBe(false);
  });
});

describe("ensureReady", () => {
  it("resolves without throwing", async () => {
    await expect(ensureReady()).resolves.toBeUndefined();
  });
});
```

- [ ] **Step 3: Run tests to verify they fail**

```bash
bun test packages/tools/src/tools/embedder.spec.ts
```
Expected: FAIL — "Cannot find module './embedder.js'"

- [ ] **Step 4: Implement `embedder.ts`**

```typescript
// packages/tools/src/tools/embedder.ts
import { existsSync } from "fs";
import { homedir } from "os";
import { join } from "path";

const MODEL = "Xenova/all-MiniLM-L6-v2";

// eslint-disable-next-line @typescript-eslint/no-explicit-any
let pipelineInstance: any = null;

async function getPipeline() {
  if (pipelineInstance) return pipelineInstance;
  const { pipeline } = await import("@xenova/transformers");
  pipelineInstance = await pipeline("feature-extraction", MODEL, { quantized: true });
  return pipelineInstance;
}

export async function embed(text: string): Promise<number[]> {
  const pipe = await getPipeline();
  const output = await pipe(text, { pooling: "mean", normalize: true });
  return Array.from(output.data as Float32Array);
}

export async function isAvailable(): Promise<boolean> {
  try {
    const { env } = await import("@xenova/transformers");
    const cacheDir = env.cacheDir ?? join(homedir(), ".cache", "xenova");
    // MODEL = "Xenova/all-MiniLM-L6-v2" — join splits the path correctly on all platforms
    return existsSync(join(cacheDir, MODEL));
  } catch {
    return false;
  }
}

export async function ensureReady(): Promise<void> {
  await getPipeline();
}
```

- [ ] **Step 5: Run tests to verify they pass**

```bash
bun test packages/tools/src/tools/embedder.spec.ts
```
Expected: all tests pass

- [ ] **Step 6: Commit**

```bash
git add packages/tools/src/tools/embedder.ts packages/tools/src/tools/embedder.spec.ts packages/tools/package.json bun.lock
git commit -m "feat(tools): add embedder — Transformer.js all-MiniLM-L6-v2 wrapper with lazy load"
```

---

## Chunk 2: Reindex integration and search

### Task 4: Wire `buildChunksAndEmbeddings` into `reindexRepo`

**Files:**
- Modify: `packages/tools/src/tools/reindex.ts`

Add a call to `buildChunksAndEmbeddings()` after the `Promise.all` that writes `symbol-map.json` (around line 216–223). The call is awaited — it is not fire-and-forget.

In `reindex.ts`, locate the **first** `Promise.all` that writes all six artifacts (stack.md, shortcuts.md, symbol-map.json, doc-index.json, traceability.json, patterns.md):
```typescript
  await Promise.all([
    writeFile(senseiPath(repoPath, "stack.md"), formatStack(stack)),
    writeFile(senseiPath(repoPath, "shortcuts.md"), formatShortcuts(shortcuts)),
    writeFile(senseiPath(repoPath, "symbol-map.json"), JSON.stringify(symbolMap, null, 2)),
    writeFile(senseiPath(repoPath, "doc-index.json"), JSON.stringify(newDocIndex, null, 2)),
    writeFile(senseiPath(repoPath, "traceability.json"), JSON.stringify(traceability, null, 2)),
    ensurePatternsMd(repoPath),
  ]);
```

Immediately after the closing `]);` of that block (not after the second `Promise.all` for llms.txt/CLAUDE.md), add:
```typescript
  await buildChunksAndEmbeddings(repoPath, symbolMap, docFiles, { force });
```

The second `Promise.all` (for `generateLlmsTxt` and `generateClaudeMd`) comes after — do not place the call there. The anchor is the `if (!existsSync(senseiPath(repoPath, "llmspec.yaml")))` check that appears between the two blocks.

Also add the import at the top of the file:
```typescript
import { buildChunksAndEmbeddings } from "./chunker.js";
```

- [ ] **Step 1: Add the import**

In `packages/tools/src/tools/reindex.ts`, after the last import line, add:
```typescript
import { buildChunksAndEmbeddings } from "./chunker.js";
```

- [ ] **Step 2: Add the `buildChunksAndEmbeddings` call**

After the `Promise.all([writeFile(...), ...])` block that writes stack.md, shortcuts.md, symbol-map.json, etc., add:
```typescript
  await buildChunksAndEmbeddings(repoPath, symbolMap, docFiles, { force });
```

- [ ] **Step 3: Verify existing reindex tests still pass**

```bash
bun test packages/tools/src/tools/reindex.spec.ts
```
Expected: all existing tests pass

- [ ] **Step 4: Commit**

```bash
git add packages/tools/src/tools/reindex.ts
git commit -m "feat(tools): wire buildChunksAndEmbeddings into reindexRepo after symbol-map write"
```

---

### Task 5: Search tool

**Files:**
- Create: `packages/tools/src/tools/search.ts`
- Create: `packages/tools/src/tools/search.spec.ts`

Implements unified search: symbol layer + BM25 layer + semantic layer, merged via RRF (k=60).

**Symbol search** operates on `symbolMap` (in-memory):
- Exact match on any L0 entry where `l0.includes(query)` and symbol name (pre-`(`) equals query → score 1.0
- Prefix match: name starts with query → score 0.8
- Substring: name includes query → score 0.5
- Returns `Array<{ id: string; score: number }>`

**BM25 search**: calls `scoreBM25()` using loaded chunks.json data.

**Semantic search**: embeds query, cosine similarity against all vectors in embeddings.json.

**RRF merge**:
```
score(chunk) = Σ layers  1 / (60 + rank_in_layer_results)
```
Rank is 0-indexed position in each layer's result array. Only chunks that appear in at least one layer are included. Deduplicate by id, sort descending.

**Zero-hit guard**: module-level `let reindexInProgress = false`. If all three layers return 0 results, and `!reindexInProgress`, set flag, call `reindexRepo(repoPath)` as unawaited Promise that clears flag on completion. Return a plain string message.

Module-level state for loaded data (cache the loaded chunks/embeddings between calls in same process):
```typescript
let cachedChunks: ChunksFile | null = null;
let cachedEmbeddings: EmbeddingsFile | null = null;
```

**Exported function**:
```typescript
export async function search(
  repoPath: string,
  query: string,
  options?: { top?: number; type?: "all" | "symbol" | "fulltext" | "semantic" }
): Promise<SearchResult[] | string>
```

Returns `SearchResult[]` normally, or a string message on zero hits.

```typescript
export interface SearchResult {
  id: string;
  file: string;
  type: "symbol" | "doc";
  excerpt: string;      // chunk text, max 200 chars
  score: number;        // RRF score
  matchedBy: Array<"symbol" | "bm25" | "semantic">;
}
```

- [ ] **Step 1: Write the failing tests**

```typescript
// packages/tools/src/tools/search.spec.ts
import { describe, it, expect, vi, beforeEach, afterEach } from "vitest";
import { mkdirSync, writeFileSync, rmSync } from "fs";
import { join } from "path";
import { search } from "./search.js";

const TMP = "/tmp/sensei-test-search";

// Fixed vectors: auth gets high similarity to "authenticate" query, home gets low
const AUTH_VECTOR = new Array(384).fill(0).map((_, i) => i < 192 ? 0.1 : 0.0);
const HOME_VECTOR = new Array(384).fill(0).map((_, i) => i >= 192 ? 0.1 : 0.0);
const QUERY_VECTOR = new Array(384).fill(0).map((_, i) => i < 192 ? 0.1 : 0.0); // matches auth

vi.mock("./embedder.js", () => ({
  embed: vi.fn().mockResolvedValue(QUERY_VECTOR),
  isAvailable: vi.fn().mockResolvedValue(true),
}));

vi.mock("./reindex.js", () => ({
  reindexRepo: vi.fn().mockResolvedValue({ added: 0, updated: 0, removed: 0, unchanged: 0, skipped: 0, total: 0, forced: false }),
}));

const symbolMap = {
  "src/auth.ts": {
    L0: ["login(email: string, password: string): Promise<User | null>"],
    L1: ["// Authenticate user\n// login(email: string, password: string): Promise<User | null>"],
    L2: [],
  },
};

const chunksData = {
  version: 1,
  corpusSize: 2,
  avgChunkLength: 6,
  chunks: {
    "src/auth.ts:login": {
      file: "src/auth.ts",
      type: "symbol",
      text: "login(email: string, password: string): Promise<User | null>\nAuthenticate user",
      contentHash: "abc123",
      tf: { login: 1, email: 1, authenticate: 1, user: 1 },
    },
    "src/home.ts:render": {
      file: "src/home.ts",
      type: "symbol",
      text: "render(): void\nRender the home page",
      contentHash: "def456",
      tf: { render: 1, home: 1, page: 1 },
    },
  },
};

const embeddingsData = {
  version: 1,
  model: "Xenova/all-MiniLM-L6-v2",
  dimensions: 384,
  vectors: {
    "src/auth.ts:login": AUTH_VECTOR,
    "src/home.ts:render": HOME_VECTOR,
  },
};

beforeEach(() => {
  mkdirSync(join(TMP, ".sensei"), { recursive: true });
  writeFileSync(join(TMP, ".sensei/symbol-map.json"), JSON.stringify(symbolMap));
  writeFileSync(join(TMP, ".sensei/chunks.json"), JSON.stringify(chunksData));
  writeFileSync(join(TMP, ".sensei/embeddings.json"), JSON.stringify(embeddingsData));
});
afterEach(() => rmSync(TMP, { recursive: true, force: true }));

describe("search — symbol layer", () => {
  it("returns src/auth.ts for 'login' query", async () => {
    const results = await search(TMP, "login", { type: "symbol" });
    expect(Array.isArray(results)).toBe(true);
    const arr = results as import("./search.js").SearchResult[];
    expect(arr.some(r => r.file === "src/auth.ts")).toBe(true);
    expect(arr[0].matchedBy).toContain("symbol");
  });
});

describe("search — BM25 layer", () => {
  it("returns auth chunk for 'authenticate user' query", async () => {
    const results = await search(TMP, "authenticate user", { type: "fulltext" });
    const arr = results as import("./search.js").SearchResult[];
    expect(arr.some(r => r.id === "src/auth.ts:login")).toBe(true);
    expect(arr[0].matchedBy).toContain("bm25");
  });
});

describe("search — semantic layer", () => {
  it("returns results with semantic in matchedBy when embeddings present", async () => {
    const results = await search(TMP, "verify identity", { type: "semantic" });
    const arr = results as import("./search.js").SearchResult[];
    expect(arr.length).toBeGreaterThan(0);
    expect(arr[0].matchedBy).toContain("semantic");
  });
});

describe("search — RRF fusion", () => {
  it("chunk appearing in 2 layers ranks above chunk in 1 layer", async () => {
    // auth:login appears in symbol + bm25 + semantic; home:render appears in fewer
    const results = await search(TMP, "login authenticate", { type: "all" });
    const arr = results as import("./search.js").SearchResult[];
    expect(arr.length).toBeGreaterThan(0);
    const authIdx = arr.findIndex(r => r.id === "src/auth.ts:login");
    const homeIdx = arr.findIndex(r => r.id === "src/home.ts:render");
    if (homeIdx !== -1) {
      expect(authIdx).toBeLessThan(homeIdx);
    }
  });

  it("result has excerpt max 200 chars", async () => {
    const results = await search(TMP, "login", { type: "all" });
    const arr = results as import("./search.js").SearchResult[];
    arr.forEach(r => expect(r.excerpt.length).toBeLessThanOrEqual(200));
  });
});

describe("search — zero-hit reindex guard", () => {
  // vi.resetModules() gives each test a fresh search module instance, resetting
  // module-level state: reindexInProgress = false, cachedChunks = null,
  // cachedEmbeddings = null. Without this, state bleeds between tests.
  beforeEach(() => {
    vi.resetModules();
    // Set up empty repo so all layers return 0 hits
    writeFileSync(join(TMP, ".sensei/symbol-map.json"), JSON.stringify({}));
    rmSync(join(TMP, ".sensei/chunks.json"), { force: true });
    rmSync(join(TMP, ".sensei/embeddings.json"), { force: true });
  });

  it("calls reindexRepo exactly once on zero hits and returns string", async () => {
    // Import fresh module instance after resetModules()
    const { search: freshSearch } = await import("./search.js");
    const { reindexRepo } = await import("./reindex.js");
    const mockReindex = vi.mocked(reindexRepo);
    mockReindex.mockClear();

    const result = await freshSearch(TMP, "nonexistent_xyz_query_abc");
    expect(typeof result).toBe("string");
    // Give a microtask tick for the unawaited promise to register
    await new Promise(r => setTimeout(r, 10));
    expect(mockReindex).toHaveBeenCalledTimes(1);
  });

  it("reindexInProgress guard prevents second reindex call", async () => {
    // Import fresh module instance after resetModules()
    const { search: freshSearch } = await import("./search.js");
    const { reindexRepo } = await import("./reindex.js");
    const mockReindex = vi.mocked(reindexRepo);
    // Never resolves — holds reindexInProgress = true for entire test
    mockReindex.mockImplementation(() => new Promise(() => {}));
    mockReindex.mockClear();

    // Two concurrent zero-hit calls hit the same module-level flag
    const [r1, r2] = await Promise.all([
      freshSearch(TMP, "nonexistent_xyz_1"),
      freshSearch(TMP, "nonexistent_xyz_2"),
    ]);
    expect(typeof r1).toBe("string");
    expect(typeof r2).toBe("string");
    // Only the first triggers reindex; the second sees the flag already set
    expect(mockReindex).toHaveBeenCalledTimes(1);
  });
});
```

- [ ] **Step 2: Run tests to verify they fail**

```bash
bun test packages/tools/src/tools/search.spec.ts
```
Expected: FAIL — "Cannot find module './search.js'"

- [ ] **Step 3: Implement `search.ts`**

```typescript
// packages/tools/src/tools/search.ts
import { readFile } from "fs/promises";
import { existsSync } from "fs";
import type { SymbolMap } from "@sensei/shared";
import { senseiPath } from "@sensei/shared";
import { scoreBM25 } from "./bm25.js";
import { embed } from "./embedder.js";

export interface SearchResult {
  id: string;
  file: string;
  type: "symbol" | "doc";
  excerpt: string;
  score: number;
  matchedBy: Array<"symbol" | "bm25" | "semantic">;
}

interface Chunk {
  file: string;
  type: "symbol" | "doc";
  text: string;
  contentHash: string;
  tf: Record<string, number>;
}
interface ChunksFile {
  version: number;
  corpusSize: number;
  avgChunkLength: number;
  chunks: Record<string, Chunk>;
}
interface EmbeddingsFile {
  version: number;
  model: string;
  dimensions: number;
  vectors: Record<string, number[]>;
}

// Module-level cache (reset between tests via module reload)
let cachedChunks: ChunksFile | null = null;
let cachedEmbeddings: EmbeddingsFile | null = null;
let reindexInProgress = false;

function cosineSimilarity(a: number[], b: number[]): number {
  let dot = 0, magA = 0, magB = 0;
  for (let i = 0; i < a.length; i++) {
    dot += a[i] * b[i];
    magA += a[i] * a[i];
    magB += b[i] * b[i];
  }
  const denom = Math.sqrt(magA) * Math.sqrt(magB);
  return denom === 0 ? 0 : dot / denom;
}

function symbolNameFromL0(l0: string): string {
  return l0.replace(/^(?:export\s+)?(?:async\s+)?(?:function|class|const|type|interface|enum)\s+/, "")
           .split(/[(<: ]/)[0].trim();
}

async function loadChunks(repoPath: string): Promise<ChunksFile | null> {
  if (cachedChunks) return cachedChunks;
  const path = senseiPath(repoPath, "chunks.json");
  if (!existsSync(path)) return null;
  try {
    cachedChunks = JSON.parse(await readFile(path, "utf-8")) as ChunksFile;
    return cachedChunks;
  } catch { return null; }
}

async function loadEmbeddings(repoPath: string): Promise<EmbeddingsFile | null> {
  if (cachedEmbeddings) return cachedEmbeddings;
  const path = senseiPath(repoPath, "embeddings.json");
  if (!existsSync(path)) return null;
  try {
    cachedEmbeddings = JSON.parse(await readFile(path, "utf-8")) as EmbeddingsFile;
    return cachedEmbeddings;
  } catch { return null; }
}

async function loadSymbolMap(repoPath: string): Promise<SymbolMap> {
  const path = senseiPath(repoPath, "symbol-map.json");
  if (!existsSync(path)) return {};
  try { return JSON.parse(await readFile(path, "utf-8")) as SymbolMap; } catch { return {}; }
}

function symbolSearch(query: string, symbolMap: SymbolMap): Array<{ id: string; score: number; file: string; type: "symbol" | "doc" }> {
  const q = query.toLowerCase();
  const results: Array<{ id: string; score: number; file: string; type: "symbol" | "doc" }> = [];

  for (const [file, symbols] of Object.entries(symbolMap)) {
    for (const l0 of symbols.L0 ?? []) {
      const name = symbolNameFromL0(l0).toLowerCase();
      if (!name) continue;
      let score = 0;
      if (name === q) score = 1.0;
      else if (name.startsWith(q)) score = 0.8;
      else if (name.includes(q)) score = 0.5;
      if (score > 0) {
        results.push({ id: `${file}:${symbolNameFromL0(l0)}`, score, file, type: "symbol" });
      }
    }
  }

  return results.sort((a, b) => b.score - a.score);
}

function rrfMerge(
  layers: Array<Array<{ id: string }>>,
  chunkData: Record<string, Chunk>,
  symbolLayerResults: Array<{ id: string; file: string; type: "symbol" | "doc" }>,
  layerNames: Array<"symbol" | "bm25" | "semantic">
): SearchResult[] {
  const RRF_K = 60;
  const scores: Record<string, number> = {};
  const matchedBy: Record<string, Set<"symbol" | "bm25" | "semantic">> = {};

  for (let i = 0; i < layers.length; i++) {
    const layer = layers[i];
    const name = layerNames[i];
    for (let rank = 0; rank < layer.length; rank++) {
      const id = layer[rank].id;
      scores[id] = (scores[id] ?? 0) + 1 / (RRF_K + rank);
      if (!matchedBy[id]) matchedBy[id] = new Set();
      matchedBy[id].add(name);
    }
  }

  // Build result lookup for symbol layer (has file/type info not in chunks.json)
  const symbolById = Object.fromEntries(symbolLayerResults.map(r => [r.id, r]));

  return Object.entries(scores)
    .sort((a, b) => b[1] - a[1])
    .map(([id, score]) => {
      const chunk = chunkData[id];
      const sym = symbolById[id];
      const file = chunk?.file ?? sym?.file ?? id.split(":")[0];
      const type: "symbol" | "doc" = chunk?.type ?? sym?.type ?? "symbol";
      const text = chunk?.text ?? "";
      return {
        id,
        file,
        type,
        excerpt: text.slice(0, 200),
        score,
        matchedBy: Array.from(matchedBy[id] ?? []),
      };
    });
}

export async function search(
  repoPath: string,
  query: string,
  options?: { top?: number; type?: "all" | "symbol" | "fulltext" | "semantic" }
): Promise<SearchResult[] | string> {
  const top = options?.top ?? 10;
  const type = options?.type ?? "all";

  const [symbolMap, chunks, embeddings] = await Promise.all([
    loadSymbolMap(repoPath),
    loadChunks(repoPath),
    loadEmbeddings(repoPath),
  ]);

  const layers: Array<Array<{ id: string }>> = [];
  const layerNames: Array<"symbol" | "bm25" | "semantic"> = [];
  let symbolLayerResults: Array<{ id: string; file: string; type: "symbol" | "doc" }> = [];

  // Symbol layer
  if (type === "all" || type === "symbol") {
    symbolLayerResults = symbolSearch(query, symbolMap);
    if (symbolLayerResults.length > 0) {
      layers.push(symbolLayerResults);
      layerNames.push("symbol");
    }
  }

  // BM25 layer
  if ((type === "all" || type === "fulltext") && chunks) {
    const bm25Results = scoreBM25(query, chunks.chunks, chunks.corpusSize, chunks.avgChunkLength);
    if (bm25Results.length > 0) {
      layers.push(bm25Results);
      layerNames.push("bm25");
    }
  }

  // Semantic layer
  if ((type === "all" || type === "semantic") && embeddings && Object.keys(embeddings.vectors).length > 0) {
    try {
      const queryVec = await embed(query);
      const semResults = Object.entries(embeddings.vectors)
        .map(([id, vec]) => ({ id, score: cosineSimilarity(queryVec, vec) }))
        .filter(r => r.score > 0.01)
        .sort((a, b) => b.score - a.score);
      if (semResults.length > 0) {
        layers.push(semResults);
        layerNames.push("semantic");
      }
    } catch {
      // Semantic layer unavailable — skip and warn
      console.warn("Semantic search unavailable — run sensei index to generate embeddings");
    }
  }

  if (layers.length === 0) {
    // Zero hits across all layers
    if (!reindexInProgress) {
      reindexInProgress = true;
      import("./reindex.js")
        .then(({ reindexRepo }) => reindexRepo(repoPath))
        .finally(() => { reindexInProgress = false; });
      return "No results found. Index may be stale — reindexing in background, retry in a moment.";
    }
    return "No results found. Reindex already in progress — retry in a moment.";
  }

  const chunkData = chunks?.chunks ?? {};
  const merged = rrfMerge(layers, chunkData, symbolLayerResults, layerNames);
  return merged.slice(0, top);
}
```

- [ ] **Step 4: Run tests to verify they pass**

```bash
bun test packages/tools/src/tools/search.spec.ts
```
Expected: all tests pass

- [ ] **Step 5: Commit**

```bash
git add packages/tools/src/tools/search.ts packages/tools/src/tools/search.spec.ts
git commit -m "feat(tools): add unified search — symbol + BM25 + semantic via RRF, zero-hit reindex guard"
```

---

## Chunk 3: Wiring into MCP, CLI, and watch command

### Task 6: Export `search` and register MCP tool

**Files:**
- Modify: `packages/tools/src/index.ts`
- Modify: `packages/mcp/src/index.ts`

**`packages/tools/src/index.ts`**: Add at the end of existing exports:
```typescript
export { search } from "./tools/search.js";
export type { SearchResult } from "./tools/search.js";
```

**`packages/mcp/src/index.ts`**: Register `search` tool. The file uses a module-level constant `REPO` (set from `process.env.REPO_PATH ?? process.cwd()`) — NOT `repoPath`. All existing tool handlers reference `REPO`. Read the file first to confirm the variable name and copy the existing registration pattern exactly.

MCP tool registration (use `REPO`, matching all other tools in the file):
```typescript
server.tool(
  "search",
  "Search the indexed repo using symbol, BM25, and semantic layers. Returns ranked results.",
  {
    query: z.string().describe("Search query"),
    top: z.number().optional().describe("Max results (default: 10)"),
    type: z.enum(["all", "symbol", "fulltext", "semantic"]).optional().describe("Search layer (default: all)"),
  },
  async ({ query, top, type }) => {
    const { search } = await import("@sensei/tools");
    const result = await search(REPO, query, { top, type });
    const text = typeof result === "string" ? result : JSON.stringify(result);
    return { content: [{ type: "text", text }] };
  }
);
```

- [ ] **Step 1: Read `packages/mcp/src/index.ts` to confirm `REPO` variable name and tool registration pattern**

Read the file. Confirm: the repo path variable, how other tools import from `@sensei/tools`, where to add the new tool registration.

- [ ] **Step 2: Add export to `packages/tools/src/index.ts`**

- [ ] **Step 3: Register `search` tool in `packages/mcp/src/index.ts`**

- [ ] **Step 4: Build tools and mcp packages to verify no type errors**

```bash
cd packages/tools && bun run build
cd ../mcp && bun run build
```
Expected: no errors

- [ ] **Step 5: Commit**

```bash
git add packages/tools/src/index.ts packages/mcp/src/index.ts
git commit -m "feat(mcp): register search tool — query + top + type params, RRF-ranked results"
```

---

### Task 7: `sensei watch` command

**Files:**
- Create: `packages/cli/src/commands/watch.ts`
- Modify: `packages/cli/src/cli.ts`
- Modify: `packages/cli/package.json` — add `chokidar` dependency

**`watch.ts`** behaviour:
- Watches `src/`, `docs/`, `package.json` relative to `repoPath` (these are the defaults)
- Debounce: 500ms quiet period (use `setTimeout`/`clearTimeout`)
- On change: if a reindex is already in flight, skip. Otherwise, call `reindexRepo(repoPath)` and print `reindexed N files (Xms)`.
- Does NOT watch `.sensei/` directory
- Runs as foreground process
- SIGINT: cancel debounce timer, await in-flight reindex (if any), call `watcher.close()`, print `"Watch stopped."`

```typescript
// packages/cli/src/commands/watch.ts
import chokidar from "chokidar";
import { join } from "path";
import { reindexRepo } from "@sensei/tools";

export async function watch(repoPath: string): Promise<void> {
  const watched = [
    join(repoPath, "src"),
    join(repoPath, "docs"),
    join(repoPath, "package.json"),
  ].filter(p => {
    const { existsSync } = require("fs");
    return existsSync(p);
  });

  // ... (see full implementation below)
}
```

Full implementation:

```typescript
// packages/cli/src/commands/watch.ts
import chokidar from "chokidar";
import { join } from "path";
import { existsSync } from "fs";
import { reindexRepo } from "@sensei/tools";

export async function watch(repoPath: string): Promise<void> {
  const watchTargets = [
    join(repoPath, "src"),
    join(repoPath, "docs"),
    join(repoPath, "package.json"),
  ].filter(p => existsSync(p));

  if (watchTargets.length === 0) {
    console.log("Nothing to watch — no src/, docs/, or package.json found.");
    return;
  }

  let debounceTimer: ReturnType<typeof setTimeout> | null = null;
  let reindexPromise: Promise<void> | null = null;

  const watcher = chokidar.watch(watchTargets, {
    ignored: [
      /\.sensei\//,
      /node_modules/,
      /\.git\//,
    ],
    ignoreInitial: true,
    persistent: true,
  });

  function triggerReindex() {
    if (debounceTimer) clearTimeout(debounceTimer);
    debounceTimer = setTimeout(async () => {
      debounceTimer = null;
      if (reindexPromise) return; // skip — reindex in flight
      const start = Date.now();
      reindexPromise = reindexRepo(repoPath)
        .then(summary => {
          const changed = summary.added + summary.updated + summary.removed;
          const elapsed = Date.now() - start;
          console.log(`reindexed ${changed} files (${elapsed}ms)`);
        })
        .catch(err => console.error("reindex error:", err.message))
        .finally(() => { reindexPromise = null; });
      await reindexPromise;
    }, 500);
  }

  watcher.on("change", triggerReindex);
  watcher.on("add", triggerReindex);
  watcher.on("unlink", triggerReindex);

  console.log(`Watching ${watchTargets.map(p => p.replace(repoPath + "/", "")).join(", ")}... (Ctrl+C to stop)`);

  await new Promise<void>(resolve => {
    process.on("SIGINT", async () => {
      if (debounceTimer) clearTimeout(debounceTimer);
      if (reindexPromise) await reindexPromise;
      await watcher.close();
      console.log("Watch stopped.");
      resolve();
    });
  });
}
```

**`packages/cli/src/cli.ts`** changes:
1. Add `repo: { type: "string" }` to the `parseArgs` options block (after `verbose`)
2. Add `watch` to HELP text
3. Add `case "watch"` to switch:

```typescript
case "watch": {
  const { watch } = await import("./commands/watch.js");
  const repo = values.repo ?? repoRoot;
  await watch(repo);
  break;
}
```

- [ ] **Step 1: Add `chokidar` to `packages/cli/package.json`**

```json
"chokidar": "^3.6.0"
```
Then: `bun install`

- [ ] **Step 2: Create `packages/cli/src/commands/watch.ts`**

(Use the full implementation shown above)

- [ ] **Step 3: Modify `packages/cli/src/cli.ts`**

Add `repo: { type: "string" }` to parseArgs options (after `verbose`).

Add to HELP text (after `index` section):
```
watch:
  --repo <path>            Repo to watch (default: auto-detected repo root)
```

Add case `"watch"` before the `default:` case.

- [ ] **Step 4: Build CLI to verify no type errors**

```bash
cd packages/cli && bun run build
```
Expected: no type errors

- [ ] **Step 5: Commit**

```bash
git add packages/cli/src/commands/watch.ts packages/cli/src/cli.ts packages/cli/package.json bun.lock
git commit -m "feat(cli): add sensei watch — chokidar debounce, 500ms, SIGINT cleanup, overlap guard"
```

---

## Chunk 4: E2E test and traceability updates

### Task 8: E2E smoke test

**Files:**
- Create: `packages/tools/src/tools/search.e2e.ts`

This test runs `reindexRepo()` on the actual sensei repo and verifies search returns real results. It is slow (runs the real embedder) and should be run manually or in CI with a separate script.

**Important:** This test requires the Transformer.js model to be downloaded. Skip gracefully if model unavailable.

```typescript
// packages/tools/src/tools/search.e2e.ts
import { describe, it, expect } from "vitest";
import { reindexRepo } from "./reindex.js";
import { search } from "./search.js";
import { isAvailable } from "./embedder.js";
import { join } from "path";
import { existsSync } from "fs";

// Auto-detect sensei repo root (two levels up from packages/tools/src/tools/)
const SENSEI_ROOT = join(import.meta.dirname, "../../../../");

describe("search e2e", () => {
  it("reindex + search returns reindex.ts for 'reindex repository' query", async () => {
    if (!existsSync(join(SENSEI_ROOT, ".git"))) {
      console.warn("E2E: not in a git repo, skipping");
      return;
    }

    await reindexRepo(SENSEI_ROOT);

    const results = await search(SENSEI_ROOT, "reindex repository");
    expect(typeof results).not.toBe("string");
    const arr = results as Awaited<ReturnType<typeof search>> & Array<unknown>;
    expect(arr.length).toBeGreaterThan(0);

    const hasReindexFile = (arr as Array<{ file: string }>).slice(0, 3).some(r =>
      r.file.includes("reindex")
    );
    expect(hasReindexFile).toBe(true);

    const modelAvail = await isAvailable();
    const firstResult = arr[0] as { matchedBy: string[] };
    const hasBm25OrSymbol = firstResult.matchedBy.includes("bm25") || firstResult.matchedBy.includes("symbol");
    expect(hasBm25OrSymbol).toBe(true);

    if (modelAvail) {
      console.log("Semantic search was available — full e2e ran.");
    } else {
      console.log("Semantic unavailable — symbol + BM25 layers verified only.");
    }
  }, 120_000); // 2-minute timeout for model download
});
```

- [ ] **Step 1: Create `search.e2e.ts`**

- [ ] **Step 2: Run e2e test manually**

```bash
bun test packages/tools/src/tools/search.e2e.ts --timeout 120000
```
Expected: passes (reindex.ts in top 3 for "reindex repository")

- [ ] **Step 3: Commit**

```bash
git add packages/tools/src/tools/search.e2e.ts
git commit -m "test(tools): add search e2e — reindex sensei repo, verify 'reindex repository' → reindex.ts in top 3"
```

---

### Task 9: Update traceability

**Files:**
- Modify: `docs/traceability.yaml`

Update `multi-modal-search` item status to `done` and add code entries.

In `features.indexing.items`, change:
```yaml
      - id: multi-modal-search
        section: "#multi-modal-search"
        status: planned
```
to:
```yaml
      - id: multi-modal-search
        section: "#multi-modal-search"
        status: done
```

In the `code:` section (currently `code: {}`), replace with:
```yaml
code:
  packages/tools/src/tools/chunker.ts:
    implements-design: [indexer]
    status: done
  packages/tools/src/tools/bm25.ts:
    implements-design: [indexer]
    status: done
  packages/tools/src/tools/embedder.ts:
    implements-design: [indexer, local-model-indexer]
    status: done
  packages/tools/src/tools/search.ts:
    implements-design: [indexer]
    status: done
  packages/cli/src/commands/watch.ts:
    implements-design: [incremental-indexer]
    status: done
```

- [ ] **Step 1: Update `docs/traceability.yaml`**

- [ ] **Step 2: Commit**

```bash
git add docs/traceability.yaml
git commit -m "docs: mark multi-modal-search done, add code traceability entries"
```

---

## Final verification

- [ ] Run all unit tests:

```bash
bun test packages/tools/src/tools/chunker.spec.ts packages/tools/src/tools/bm25.spec.ts packages/tools/src/tools/embedder.spec.ts packages/tools/src/tools/search.spec.ts
```
Expected: all pass

- [ ] Run full test suite:

```bash
bun test
```
Expected: no regressions

- [ ] Run `sensei index` on the repo to verify chunks.json and embeddings.json are written:

```bash
sensei index
ls .sensei/chunks.json .sensei/embeddings.json
```
Expected: both files exist

- [ ] Test `sensei watch` starts and prints watching message:

```bash
sensei watch &
sleep 1
kill %1
```
Expected: "Watching src/, docs/..." then "Watch stopped."
