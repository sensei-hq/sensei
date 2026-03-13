import { describe, it, expect, vi, beforeEach } from "vitest";
import { upsertSymbols, upsertDocs } from "./supabase-index-writer.js";

const mockUpsert = vi.fn().mockReturnValue({ error: null });
const mockFrom = vi.fn(() => ({ upsert: mockUpsert }));
const mockClient = { from: mockFrom } as any;

describe("upsertSymbols", () => {
  beforeEach(() => vi.clearAllMocks());

  it("upserts symbol rows keyed by (repo_id, file_path)", async () => {
    const symbolMap = {
      "src/a.ts": { L0: ["foo"], L1: ["function foo()"], L2: ["function foo() — does foo"] },
    };
    await upsertSymbols(mockClient, "repo-1", symbolMap);
    const rows = mockUpsert.mock.calls[0][0];
    expect(rows[0]).toMatchObject({ repo_id: "repo-1", file_path: "src/a.ts", l0: ["foo"] });
  });

  it("skips upsert when symbolMap is empty", async () => {
    await upsertSymbols(mockClient, "repo-1", {});
    expect(mockUpsert).not.toHaveBeenCalled();
  });
});

describe("upsertDocs", () => {
  beforeEach(() => vi.clearAllMocks());

  it("upserts doc coverage rows", async () => {
    const traceability = [
      { docPath: "docs/design/01.md", covers: ["src/a.ts"], autoDetected: false },
    ];
    await upsertDocs(mockClient, "repo-1", traceability);
    const rows = mockUpsert.mock.calls[0][0];
    expect(rows[0]).toMatchObject({ repo_id: "repo-1", doc_path: "docs/design/01.md", covers: ["src/a.ts"] });
  });
});
