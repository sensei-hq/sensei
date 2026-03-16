// packages/server/src/tools/get-lib-docs-fallback.spec.ts
import { describe, it, expect, vi } from "vitest";
import { getLibDocsTool } from "./get-lib-docs.js";

function makeDb(overrides: Record<string, unknown> = {}) {
  const rpcFn = vi.fn().mockResolvedValue({ data: [], error: null });
  const fromResult = {
    select: vi.fn().mockReturnThis(),
    eq: vi.fn().mockReturnThis(),
    maybeSingle: vi.fn().mockResolvedValue({ data: { shared_lib_id: "shared-123" } }),
    or: vi.fn().mockReturnThis(),
    order: vi.fn().mockReturnThis(),
    // limit must be defined — keyword fallback calls .order(...).limit(N) and awaits the result
    limit: vi.fn().mockResolvedValue({ data: [], error: null }),
    is: vi.fn().mockReturnThis(),
  };
  return {
    rpc: rpcFn,
    from: vi.fn().mockReturnValue(fromResult),
    schema: vi.fn().mockReturnValue({ from: vi.fn().mockReturnValue(fromResult) }),
    _rpcFn: rpcFn,
    _fromResult: fromResult,
    ...overrides,
  };
}

describe("getLibDocsTool — keyword fallback", () => {
  it("falls back to ILIKE search when vector RPC returns 0 results", async () => {
    const keywordRows = [
      { title: "Button", url: "https://x.com/btn", description: "A button", content: null, source_type: "llms.txt", component: null, local_path: null },
    ];
    const db = makeDb();
    // RPC returns empty (no embeddings) — already the default mock
    // keyword path: .or() → .order() → .limit() resolves with data
    db._fromResult.limit.mockResolvedValue({ data: keywordRows, error: null });

    const backend = { embed: vi.fn().mockResolvedValue([0.1, 0.2]) } as any;
    const result = await getLibDocsTool(db as any, backend, "repo-id", "mylib", { query: "button" });

    expect(result.sections).toHaveLength(1);
    expect(result.sections[0].title).toBe("Button");
  });

  it("uses vector search when RPC returns results", async () => {
    const vectorRows = [
      { title: "Vector Result", url: null, description: "Found via vector", content: null, source_type: "llms.txt", component: null, local_path: null, similarity: 0.9 },
    ];
    const db = makeDb();
    db._rpcFn.mockResolvedValue({ data: vectorRows, error: null });

    const backend = { embed: vi.fn().mockResolvedValue([0.1, 0.2]) } as any;
    const result = await getLibDocsTool(db as any, backend, "repo-id", "mylib", { query: "vector" });

    expect(result.sections[0].title).toBe("Vector Result");
  });
});
