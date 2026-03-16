// packages/engine/src/lib/lib-indexer.spec.ts
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

// Build a mock Supabase client that records operations
const makeMockDb = () => {
  const insertedBatches: unknown[] = [];
  let deleteChain = { eq: vi.fn() };
  deleteChain.eq = vi.fn().mockReturnValue({ eq: vi.fn().mockResolvedValue({ error: null }) });

  return {
    _insertedBatches: insertedBatches,
    from: vi.fn().mockImplementation((table: string) => ({
      delete: vi.fn().mockReturnValue(deleteChain),
      insert: vi.fn().mockImplementation((rows: unknown) => {
        insertedBatches.push(rows);
        return Promise.resolve({ error: null });
      }),
    })),
  };
};

describe("LibIndexer", () => {
  it("deletes existing rows then inserts N rows, calls embed N times", async () => {
    const db = makeMockDb();
    const backend = makeMockBackend();
    const indexer = new LibIndexer(db as any, backend);

    const entry: LibEntry = { name: "rokkit", source_type: "llms.txt", base_url: "https://x.com/llms.txt" };
    const pages: DocPage[] = [
      { title: "Button", url: "https://rokkit.dev/button", summary: "A button", content: "A button component docs.", sourceType: "llms.txt" },
      { title: "Input", url: "https://rokkit.dev/input", summary: "An input", content: "An input component docs.", sourceType: "llms.txt" },
    ];

    const result = await indexer.index("repo-123", entry, pages);

    expect(result.sectionsIndexed).toBe(2);
    expect(backend.embed).toHaveBeenCalledTimes(2);
    // delete was called before insert
    expect(db.from).toHaveBeenCalledWith("lib_doc_sections");
    expect(db._insertedBatches).toHaveLength(1);
    const inserted = db._insertedBatches[0] as unknown[];
    expect(inserted).toHaveLength(2);
  });

  it("embeds description for llms.txt; first 512 chars of content for http/local", async () => {
    const backend = makeMockBackend();
    const db = makeMockDb();
    const indexer = new LibIndexer(db as any, backend);

    const content = "X".repeat(600);
    const pages: DocPage[] = [
      { title: "Usage", url: "https://x.com", summary: "Short desc", content, sourceType: "http" },
    ];
    const entry: LibEntry = { name: "kavach", source_type: "http", base_url: "https://kavach.dev" };

    await indexer.index("repo-123", entry, pages);

    expect(backend.embed).toHaveBeenCalledWith("X".repeat(512));
  });

  it("embed input for non-llms.txt types uses content.slice(0,512)", async () => {
    const backend = makeMockBackend();
    const db = makeMockDb();
    const indexer = new LibIndexer(db as any, backend);

    const pages: DocPage[] = [
      { title: "Page", url: "https://x.com", summary: "fallback desc", content: "fallback content", sourceType: "http" },
    ];

    await indexer.index("repo-123", { name: "x", source_type: "http" }, pages);
    expect(backend.embed).toHaveBeenCalledWith("fallback content");
  });
});

describe("LibIndexer.indexShared — no backend (Phase 1)", () => {
  function makeDb() {
    const insertFn = vi.fn().mockResolvedValue({ error: null });
    const deleteFn = () => ({ eq: vi.fn().mockResolvedValue({ error: null }) });
    const deleteShared = () => ({ eq: vi.fn().mockResolvedValue({ error: null }) });
    return {
      from: vi.fn((table: string) => ({
        delete: table === "shared_lib_sections" ? deleteShared : deleteFn,
        insert: insertFn,
      })),
      _insertFn: insertFn,
    };
  }

  it("inserts sections with null embedding when no backend provided", async () => {
    const db = makeDb() as any;
    const indexer = new LibIndexer(db, null);
    const pages: DocPage[] = [
      { title: "Button", summary: "A button component", content: "A button component docs.", sourceType: "llms.txt", url: "https://x.com/btn" },
      { title: "Input", summary: "A text input", content: "An input component docs.", sourceType: "llms.txt", url: "https://x.com/inp" },
    ];
    const { sectionsIndexed } = await indexer.indexShared("lib-id-123", { name: "mylib", source_type: "llms.txt", base_url: "https://x.com" }, pages);

    expect(sectionsIndexed).toBe(2);
    const insertedRows = db._insertFn.mock.calls[0][0] as any[];
    expect(insertedRows).toHaveLength(2);
    expect(insertedRows[0].embedding).toBeNull();
    expect(insertedRows[0].title).toBe("Button");
  });

  it("inserts sections with embeddings when backend provided", async () => {
    const db = makeDb() as any;
    const backend: Partial<ModelBackend> = {
      embed: vi.fn().mockResolvedValue([0.1, 0.2, 0.3]),
    };
    const indexer = new LibIndexer(db, backend as ModelBackend);
    const pages: DocPage[] = [
      { title: "Button", summary: "A button component", content: "A button component docs.", sourceType: "llms.txt", url: "https://x.com/btn" },
      { title: "Input", summary: "A text input", content: "An input component docs.", sourceType: "llms.txt", url: "https://x.com/inp" },
    ];
    await indexer.indexShared("lib-id-123", { name: "mylib", source_type: "llms.txt", base_url: "https://x.com" }, pages);

    const insertedRows = db._insertFn.mock.calls[0][0] as any[];
    expect(insertedRows[0].embedding).toEqual([0.1, 0.2, 0.3]);
    expect(backend.embed).toHaveBeenCalledTimes(2);
  });
});
