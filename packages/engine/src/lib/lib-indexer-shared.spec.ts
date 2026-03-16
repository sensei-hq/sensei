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

// Tracks all insert calls by table name
const makeMockDb = () => {
  const insertsByTable: Record<string, unknown[][]> = {};
  const _deleteEq = vi.fn().mockResolvedValue({ error: null });

  const makeInsertable = (table: string, rows: unknown) => {
    if (!insertsByTable[table]) insertsByTable[table] = [];
    insertsByTable[table].push(rows as unknown[]);
    const arr = Array.isArray(rows) ? rows : [rows];
    const mockData = arr.map((_, i) => ({ id: `mock-${table}-${i}` }));

    const result = {
      select: vi.fn().mockReturnValue({
        single: vi.fn().mockResolvedValue({ data: mockData[0] ?? null, error: null }),
        eq: vi.fn().mockResolvedValue({ data: null, error: null }),
      }),
    };

    // Make it thenable so `await db.from('x').insert(...)` works directly
    (result as any).then = (
      onfulfilled: (v: { data: typeof mockData; error: null }) => unknown,
      _onrejected?: unknown,
    ) => Promise.resolve({ data: mockData, error: null }).then(onfulfilled);

    return result;
  };

  return {
    _insertsByTable: insertsByTable,
    _deleteEq,
    from: vi.fn().mockImplementation((table: string) => ({
      delete: vi.fn().mockReturnValue({ eq: _deleteEq }),
      insert: vi.fn().mockImplementation((rows: unknown) => makeInsertable(table, rows)),
      select: vi.fn().mockReturnValue({
        eq: vi.fn().mockReturnValue({ data: null, error: null }),
      }),
    })),
  };
};

const makeEntry = (): LibEntry => ({ name: "rokkit", source_type: "llms.txt", base_url: "https://rokkit.dev" });

const makePages = (): DocPage[] => [
  {
    title: "Button",
    url: "https://rokkit.dev/button",
    summary: "A button component",
    content: "# Button\n\nFull button docs.\n\n## Props\n\nSize and variant.",
    sourceType: "llms.txt",
    component: "Components",
    sequence: 0,
  },
  {
    title: "Input",
    url: "https://rokkit.dev/input",
    summary: "An input component",
    content: "# Input\n\nFull input docs.\n\n## Props\n\nType and placeholder.",
    sourceType: "llms.txt",
    component: "Forms",
    sequence: 1,
  },
];

describe("LibIndexer.indexShared — two-level insert", () => {
  it("deletes documents_in_library (not sections) to cascade-delete sections", async () => {
    const db = makeMockDb();
    const indexer = new LibIndexer(db as any, null);
    await indexer.indexShared("lib-42", makeEntry(), makePages());

    expect(db.from).toHaveBeenCalledWith("documents_in_library");
    expect(db._deleteEq).toHaveBeenCalledWith("library_id", "lib-42");
  });

  it("inserts one row per DocPage into documents_in_library", async () => {
    const db = makeMockDb();
    const indexer = new LibIndexer(db as any, null);
    await indexer.indexShared("lib-42", makeEntry(), makePages());

    const docInserts = db._insertsByTable["documents_in_library"];
    expect(docInserts).toBeDefined();
    // May be batched as one insert([...]) or many insert(row)
    const allDocs = docInserts.flat();
    expect(allDocs).toHaveLength(2);
    const first = allDocs[0] as Record<string, unknown>;
    expect(first.library_id).toBe("lib-42");
    expect(first.title).toBe("Button");
    expect(first.summary).toBe("A button component");
    expect(first.url).toBe("https://rokkit.dev/button");
    expect(first.component).toBe("Components");
    expect(first.sequence).toBe(0);
    expect(first.source_type).toBe("llms.txt");
  });

  it("splits content into sections_in_document rows with document_id FK", async () => {
    const db = makeMockDb();
    // Mock that insert into documents_in_library returns an id
    let docInsertCall = 0;
    db.from = vi.fn().mockImplementation((table: string) => ({
      delete: vi.fn().mockReturnValue({ eq: vi.fn().mockResolvedValue({ error: null }) }),
      insert: vi.fn().mockImplementation((rows: unknown) => {
        if (table === "documents_in_library") {
          const arr = Array.isArray(rows) ? rows : [rows];
          return Promise.resolve({ data: arr.map((_, i) => ({ id: `doc-id-${docInsertCall++}-${i}` })), error: null });
        }
        if (!db._insertsByTable[table]) db._insertsByTable[table] = [];
        db._insertsByTable[table].push(rows as unknown[]);
        return Promise.resolve({ error: null });
      }),
      select: vi.fn().mockReturnThis(),
    })) as any;

    const indexer = new LibIndexer(db as any, null);
    await indexer.indexShared("lib-42", makeEntry(), makePages());

    const sectionInserts = db._insertsByTable["sections_in_document"] ?? [];
    const allSections = sectionInserts.flat() as Record<string, unknown>[];
    // Button doc: Overview + Props → 2 sections
    // Input doc: Overview + Props → 2 sections
    expect(allSections.length).toBeGreaterThanOrEqual(4);
    allSections.forEach(s => {
      expect(s.library_id).toBe("lib-42");
      expect(s.document_id).toBeDefined();
      expect(s.title).toBeTruthy();
      expect(s.content).toBeTruthy();
    });
  });

  it("returns documentsIndexed and sectionsIndexed counts", async () => {
    const db = makeMockDb();
    db.from = vi.fn().mockImplementation((table: string) => ({
      delete: vi.fn().mockReturnValue({ eq: vi.fn().mockResolvedValue({ error: null }) }),
      insert: vi.fn().mockImplementation((rows: unknown) => {
        const arr = Array.isArray(rows) ? rows : [rows];
        if (table === "documents_in_library") {
          return Promise.resolve({ data: arr.map((_, i) => ({ id: `doc-${i}` })), error: null });
        }
        return Promise.resolve({ error: null });
      }),
    })) as any;

    const indexer = new LibIndexer(db as any, null);
    const result = await indexer.indexShared("lib-42", makeEntry(), makePages());

    expect(result.documentsIndexed).toBe(2);
    expect(result.sectionsIndexed).toBeGreaterThanOrEqual(2);
  });

  it("embeds section content when backend is provided", async () => {
    const backend = makeMockBackend();
    const db = makeMockDb();
    db.from = vi.fn().mockImplementation((table: string) => ({
      delete: vi.fn().mockReturnValue({ eq: vi.fn().mockResolvedValue({ error: null }) }),
      insert: vi.fn().mockImplementation((rows: unknown) => {
        const arr = Array.isArray(rows) ? rows : [rows];
        if (table === "documents_in_library") {
          return Promise.resolve({ data: arr.map((_, i) => ({ id: `doc-${i}` })), error: null });
        }
        return Promise.resolve({ error: null });
      }),
    })) as any;

    const indexer = new LibIndexer(db as any, backend);
    await indexer.indexShared("lib-42", makeEntry(), makePages());

    // embed called at least once for sections
    expect(backend.embed).toHaveBeenCalled();
    // embed input is content.slice(0, 512)
    const call = (backend.embed as ReturnType<typeof vi.fn>).mock.calls[0][0];
    expect(typeof call).toBe("string");
    expect(call.length).toBeLessThanOrEqual(512);
  });
});
