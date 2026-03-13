import { describe, it, expect, vi } from "vitest";
import { indexRepo } from "./pipeline.js";

describe("indexRepo pipeline", () => {
  it("returns IndexResult with repoId", async () => {
    // This test uses the real Scanner and TypeScriptAdapter but a mock Supabase client.
    // Point it at a directory that contains some .ts files.
    const eqChain: any = {
      eq: vi.fn(() => eqChain),
      in: vi.fn().mockResolvedValue({ data: [], error: null }),
      then: (resolve: any) => Promise.resolve({ data: [], error: null }).then(resolve),
    };
    const mockClient = {
      from: vi.fn(() => ({
        upsert: vi.fn().mockResolvedValue({ error: null }),
        delete: vi.fn(() => ({ eq: vi.fn(() => ({ in: vi.fn().mockResolvedValue({ error: null }) })) })),
        select: vi.fn(() => eqChain),
      })),
    };

    const result = await indexRepo({
      repoPath: process.cwd(), // packages/engine itself has .ts files
      repoId: "test-pipeline",
      client: mockClient as any,
    });

    expect(result.repoId).toBe("test-pipeline");
    expect(result.symbolsUpserted).toBeGreaterThan(0);
    expect(result.filesIndexed).toBeGreaterThan(0);
    expect(result.errors).toHaveLength(0);
  });
});
