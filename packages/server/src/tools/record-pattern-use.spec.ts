import { describe, it, expect, vi } from "vitest";
import { recordPatternUse } from "./record-pattern-use.js";

describe("recordPatternUse", () => {
  it("inserts a row and returns confirmation string", async () => {
    const inserted: any[] = [];
    const mockClient = {
      schema: () => ({
        from: (table: string) => {
          expect(table).toBe("pattern_usages");
          return {
            insert: vi.fn().mockImplementation((row: any) => {
              inserted.push(row);
              return Promise.resolve({ error: null });
            }),
          };
        },
      }),
    };

    const result = await recordPatternUse(mockClient as any, "repo-1", "session-1", "mcp-tool");
    expect(result).toContain("mcp-tool");
    expect(inserted).toHaveLength(1);
    expect(inserted[0]).toMatchObject({
      repo_id: "repo-1",
      session_id: "session-1",
      pattern_name: "mcp-tool",
    });
  });

  it("works without a session_id", async () => {
    const inserted: any[] = [];
    const mockClient = {
      schema: () => ({
        from: () => ({
          insert: vi.fn().mockImplementation((row: any) => {
            inserted.push(row);
            return Promise.resolve({ error: null });
          }),
        }),
      }),
    };

    await recordPatternUse(mockClient as any, "repo-1", null, "adapter");
    expect(inserted[0].session_id).toBeNull();
  });

  it("throws when insert fails", async () => {
    const mockClient = {
      schema: () => ({
        from: () => ({
          insert: vi.fn().mockResolvedValue({ error: { message: "db down" } }),
        }),
      }),
    };

    await expect(recordPatternUse(mockClient as any, "repo-1", null, "adapter"))
      .rejects.toThrow("db down");
  });
});
