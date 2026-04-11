// packages/server/src/lib-indexer.spec.ts
import { describe, it, expect, vi, beforeEach } from "vitest";
import type { DocPage, LibEntry } from "@sensei/shared";

const mockReplaceLibDocs = vi.fn();
vi.mock("./activity-log.js", () => ({
  getActivityLog: vi.fn(() => ({ replaceLibDocs: mockReplaceLibDocs })),
}));

import { LibIndexer } from "./lib-indexer.js";

beforeEach(() => {
  mockReplaceLibDocs.mockClear();
});

describe("LibIndexer", () => {
  const entry: LibEntry = { name: "rokkit", source_type: "llms.txt", base_url: "https://rokkit.dev" };

  const pages: DocPage[] = [
    { title: "Button", url: "https://rokkit.dev/button", summary: "A button", content: "A button component docs.", sourceType: "llms.txt" },
    { title: "Input", url: "https://rokkit.dev/input", summary: "An input", content: "An input component docs.", sourceType: "llms.txt" },
  ];

  it("calls replaceLibDocs with lib name and mapped docs", async () => {
    const indexer = new LibIndexer("repo-123");
    await indexer.index(entry, pages);

    expect(mockReplaceLibDocs).toHaveBeenCalledTimes(1);
    const [libName, docs] = mockReplaceLibDocs.mock.calls[0];
    expect(libName).toBe("rokkit");
    expect(docs).toHaveLength(2);
    expect(docs[0].title).toBe("Button");
    expect(docs[1].title).toBe("Input");
  });

  it("returns sectionsIndexed equal to pages count", async () => {
    const indexer = new LibIndexer("repo-123");
    const result = await indexer.index(entry, pages);
    expect(result.sectionsIndexed).toBe(2);
  });

  it("maps sourceType from page, falling back to entry source_type", async () => {
    const indexer = new LibIndexer("repo-123");
    const mixedPages: DocPage[] = [
      { title: "Page1", url: "https://x.com", summary: "s", content: "c", sourceType: "http" },
      { title: "Page2", url: "https://x.com/2", summary: "s", content: "c", sourceType: "http" },
    ];
    const httpEntry: LibEntry = { name: "mylib", source_type: "http", base_url: "https://x.com" };
    await indexer.index(httpEntry, mixedPages);

    const [, docs] = mockReplaceLibDocs.mock.calls[0];
    expect(docs[0].sourceType).toBe("http");
    expect(docs[1].sourceType).toBe("http"); // fallback to entry.source_type
  });
});
