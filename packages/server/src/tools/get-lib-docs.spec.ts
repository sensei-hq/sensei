// packages/server/src/tools/get-lib-docs.spec.ts
import { describe, it, expect, vi, beforeEach } from "vitest";

const mockGetLibDocs = vi.fn();
vi.mock("../lib-store.js", () => ({
  getLibDocs: (...args: unknown[]) => mockGetLibDocs(...args),
}));

import { getLibDocsTool } from "./get-lib-docs.js";

beforeEach(() => {
  mockGetLibDocs.mockReset();
});

describe("getLibDocsTool", () => {
  it("returns sections mapped from shared lib store", async () => {
    mockGetLibDocs.mockReturnValue([
      { id: "1", title: "Button", url: "https://rokkit.dev/button", localPath: null, summary: "A button", content: "Docs.", sourceType: "llms.txt", component: "Forms" },
    ]);

    const result = await getLibDocsTool("repo-1", "rokkit", { query: "button" });

    expect(result.lib).toBe("rokkit");
    expect(result.sections).toHaveLength(1);
    expect(result.sections[0].title).toBe("Button");
    expect(result.sections[0].content).toBe("Docs.");
    expect(result.sections[0].document.component).toBe("Forms");
  });

  it("passes opts through to getLibDocs", async () => {
    mockGetLibDocs.mockReturnValue([]);
    await getLibDocsTool("repo-1", "rokkit", { component: "Forms", limit: 5 });
    expect(mockGetLibDocs).toHaveBeenCalledWith("rokkit", { component: "Forms", limit: 5 });
  });

  it("returns empty sections when no docs exist", async () => {
    mockGetLibDocs.mockReturnValue([]);
    const result = await getLibDocsTool("repo-1", "rokkit");
    expect(result.sections).toEqual([]);
  });

  it("returns empty sections on error — never throws", async () => {
    mockGetLibDocs.mockImplementation(() => { throw new Error("DB error"); });
    const result = await getLibDocsTool("repo-1", "rokkit", { query: "test" });
    expect(result.sections).toEqual([]);
  });

  it("uses localPath as url fallback when url is null", async () => {
    mockGetLibDocs.mockReturnValue([
      { id: "1", title: "Page", url: null, localPath: "/path/to/doc.md", summary: "s", content: "c", sourceType: "http", component: null },
    ]);
    const result = await getLibDocsTool("repo-1", "mylib");
    expect(result.sections[0].document.url).toBe("/path/to/doc.md");
  });
});
