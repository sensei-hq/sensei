// packages/engine/src/lib/lib-indexer-shared.spec.ts
import { describe, it, expect, vi } from "vitest";
import { LibIndexer } from "./lib-indexer.js";
import type { DocPage, LibEntry, ModelBackend } from "@sensei/shared";

const makeMockBackend = (): ModelBackend => ({
  name: "mock",
  init: vi.fn().mockResolvedValue(undefined),
  isAvailable: vi.fn().mockResolvedValue(true),
  generate: vi.fn().mockResolvedValue(""),
  embed: vi.fn().mockResolvedValue([0.1, 0.2, 0.3]),
  extract: vi.fn().mockResolvedValue({}),
});

// Build a mock Supabase client for shared_lib_sections that records operations
// and exposes _deleteEqMock for asserting delete().eq() calls
const makeMockDbShared = () => {
  const insertedBatches: unknown[] = [];
  const _deleteEqMock = vi.fn().mockResolvedValue({ error: null });

  return {
    _insertedBatches: insertedBatches,
    _deleteEqMock,
    from: vi.fn().mockImplementation((table: string) => ({
      delete: vi.fn().mockReturnValue({ eq: _deleteEqMock }),
      insert: vi.fn().mockImplementation((rows: unknown) => {
        insertedBatches.push(rows);
        return Promise.resolve({ error: null });
      }),
    })),
  };
};

const makeEntry = (source_type: "llms.txt" | "http" | "local" = "llms.txt"): LibEntry => ({
  name: "rokkit",
  source_type,
  base_url: "https://rokkit.dev",
});

const makePages = (): DocPage[] => [
  { title: "Button", url: "https://rokkit.dev/button", description: "A button component", sourceType: "llms.txt" },
  { title: "Input", url: "https://rokkit.dev/input", description: "An input component", sourceType: "llms.txt" },
];

describe("LibIndexer.indexShared", () => {
  it("deletes existing sections for the shared lib before inserting", async () => {
    const db = makeMockDbShared();
    const backend = makeMockBackend();
    const indexer = new LibIndexer(db as any, backend);

    await indexer.indexShared("shared-lib-42", makeEntry(), makePages());

    expect(db.from).toHaveBeenCalledWith("shared_lib_sections");
    expect(db._deleteEqMock).toHaveBeenCalledWith("shared_lib_id", "shared-lib-42");
  });

  it("inserts one row per page into shared_lib_sections", async () => {
    const db = makeMockDbShared();
    const backend = makeMockBackend();
    const indexer = new LibIndexer(db as any, backend);

    await indexer.indexShared("shared-lib-42", makeEntry(), makePages());

    expect(db._insertedBatches).toHaveLength(1);
    const inserted = db._insertedBatches[0] as Record<string, unknown>[];
    expect(inserted).toHaveLength(2);
    // rows should have shared_lib_id, not repo_id
    expect(inserted[0]).toHaveProperty("shared_lib_id", "shared-lib-42");
    expect(inserted[0]).not.toHaveProperty("repo_id");
    expect(inserted[0]).toHaveProperty("title", "Button");
    expect(inserted[0]).toHaveProperty("description", "A button component");
    expect(inserted[0]).toHaveProperty("source_type", "llms.txt");
    expect(inserted[0]).toHaveProperty("embedding");
  });

  it("returns sectionsIndexed equal to number of pages", async () => {
    const db = makeMockDbShared();
    const backend = makeMockBackend();
    const indexer = new LibIndexer(db as any, backend);

    const result = await indexer.indexShared("shared-lib-42", makeEntry(), makePages());

    expect(result).toEqual({ sectionsIndexed: 2 });
  });

  it("uses description for embedding when source_type is llms.txt", async () => {
    const db = makeMockDbShared();
    const backend = makeMockBackend();
    const indexer = new LibIndexer(db as any, backend);

    const pages: DocPage[] = [
      { title: "Button", url: "https://rokkit.dev/button", description: "A button component", content: "full content", sourceType: "llms.txt" },
    ];

    await indexer.indexShared("shared-lib-42", makeEntry("llms.txt"), pages);

    expect(backend.embed).toHaveBeenCalledWith("A button component");
  });

  it("uses content.slice(0, 512) for embedding when source_type is http", async () => {
    const db = makeMockDbShared();
    const backend = makeMockBackend();
    const indexer = new LibIndexer(db as any, backend);

    const content = "H".repeat(600);
    const pages: DocPage[] = [
      { title: "Page", url: "https://x.com", description: "Short desc", content, sourceType: "http" },
    ];

    await indexer.indexShared("shared-lib-42", makeEntry("http"), pages);

    expect(backend.embed).toHaveBeenCalledWith("H".repeat(512));
  });
});
