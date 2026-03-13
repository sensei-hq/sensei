import { describe, it, expect, vi } from "vitest";
import { Indexer } from "./indexer.js";
import type { ScanResult, ParsedFile } from "@sensei/shared";

// Each call to from() gets a fresh mock object that correctly stubs all chained methods.
function makeMockClient() {
  const upsertFn = vi.fn().mockResolvedValue({ error: null });
  const inDeleteFn = vi.fn().mockResolvedValue({ error: null });
  const eqDeleteFn = vi.fn(() => ({ in: inDeleteFn }));
  const deleteChain = vi.fn(() => ({ eq: eqDeleteFn }));

  const fromFn = vi.fn(() => ({
    upsert: upsertFn,
    delete: deleteChain,
    select: vi.fn(() => ({
      eq: vi.fn(() => ({
        select: vi.fn().mockResolvedValue({ data: [], error: null }),
        in: vi.fn().mockResolvedValue({ data: [], error: null }),
      })),
    })),
  }));
  return { from: fromFn, _upsert: upsertFn };
}

describe("Indexer", () => {
  it("calls upsert for each parsed file's symbols", async () => {
    const client = makeMockClient();
    const indexer = new Indexer(client as any);

    const scan: ScanResult = {
      repoId: "repo-1",
      files: [],
      changed: ["src/a.ts"],
      deleted: [],
    };

    const parsed: ParsedFile[] = [{
      filePath: "src/a.ts",
      language: "typescript",
      symbols: [{
        name: "foo",
        kind: "function",
        signature: "(): void",
        docstring: null,
        lineStart: 1,
        lineEnd: 5,
        isExported: true,
      }],
      edges: [],
      imports: [],
    }];

    const result = await indexer.indexFiles(scan, parsed);
    expect(result.symbolsUpserted).toBe(1);
    expect(result.errors).toHaveLength(0);
  });

  it("returns zero counts when no parsed files given", async () => {
    const client = makeMockClient();
    const indexer = new Indexer(client as any);
    const scan: ScanResult = { repoId: "repo-1", files: [], changed: [], deleted: [] };
    const result = await indexer.indexFiles(scan, []);
    expect(result.symbolsUpserted).toBe(0);
    expect(result.filesIndexed).toBe(0);
  });
});
