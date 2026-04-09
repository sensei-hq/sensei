// packages/tools/src/tools/search.spec.ts
import { describe, it, expect, vi, beforeEach, afterEach } from "vitest";
import { mkdirSync, rmSync } from "fs";
import { join } from "path";

const TMP = "/tmp/sensei-test-search";

const { HOISTED_DB_FLAGS } = vi.hoisted(() => ({
  HOISTED_DB_FLAGS: { empty: false },
}));

vi.mock("./embedder.js", () => ({
  embed: vi.fn().mockResolvedValue(new Array(384).fill(0.1)),
  isAvailable: vi.fn().mockResolvedValue(true),
}));

vi.mock("./reindex.js", () => ({
  reindexRepo: vi.fn().mockResolvedValue({ added: 0, updated: 0, removed: 0, unchanged: 0, skipped: 0, total: 0, forced: false }),
}));

beforeEach(() => {
  mkdirSync(join(TMP, ".sensei"), { recursive: true });
});
afterEach(() => rmSync(TMP, { recursive: true, force: true }));

describe("search — zero-hit reindex guard", () => {
  // vi.resetModules() gives each test a fresh search module instance, resetting
  // module-level state: reindexInProgress = false.
  beforeEach(() => {
    vi.resetModules();
    HOISTED_DB_FLAGS.empty = true;
  });
  afterEach(() => { HOISTED_DB_FLAGS.empty = false; });

  it("calls reindexRepo exactly once on zero hits and returns string", async () => {
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
    const { search: freshSearch } = await import("./search.js");
    const { reindexRepo } = await import("./reindex.js");
    const mockReindex = vi.mocked(reindexRepo);
    // Never resolves — holds reindexInProgress = true for entire test
    mockReindex.mockImplementation(() => new Promise(() => {}));
    mockReindex.mockClear();

    const [r1, r2] = await Promise.all([
      freshSearch(TMP, "nonexistent_xyz_1"),
      freshSearch(TMP, "nonexistent_xyz_2"),
    ]);
    expect(typeof r1).toBe("string");
    expect(typeof r2).toBe("string");
    await new Promise(r => setTimeout(r, 10));
    expect(mockReindex).toHaveBeenCalledTimes(1);
  });
});
