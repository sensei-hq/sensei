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

const { HOISTED_QUERY_VECTOR } = vi.hoisted(() => ({
  HOISTED_QUERY_VECTOR: new Array(384).fill(0).map((_, i) => i < 192 ? 0.1 : 0.0),
}));

vi.mock("./embedder.js", () => ({
  embed: vi.fn().mockResolvedValue(HOISTED_QUERY_VECTOR),
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
