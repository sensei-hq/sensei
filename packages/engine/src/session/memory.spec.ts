// packages/engine/src/session/memory.spec.ts
import { describe, it, expect, vi } from "vitest";
import { recordMemory, closeMemory, getMemoryItems } from "./memory.js";

const memRow = {
  id: "mem-1",
  type: "decision",
  title: "Use optimistic locking",
  content: "Apply optimistic locking for invoice updates to avoid contention",
  status: "open",
  resolution: null,
  closed_at: null,
  created_at: "2026-03-13T00:00:00Z",
};

function makeSingleDb(result: any) {
  return {
    from: vi.fn().mockReturnValue({
      insert: vi.fn().mockReturnValue({
        select: vi.fn().mockReturnThis(),
        single: vi.fn().mockResolvedValue(result),
      }),
      update: vi.fn().mockReturnValue({
        eq: vi.fn().mockReturnThis(),
        select: vi.fn().mockReturnThis(),
        single: vi.fn().mockResolvedValue(result),
      }),
      select: vi.fn().mockReturnThis(),
      eq: vi.fn().mockReturnThis(),
      order: vi.fn().mockReturnThis(),
      limit: vi.fn().mockResolvedValue({ data: [memRow], error: null }),
    }),
  } as any;
}

describe("recordMemory", () => {
  it("inserts memory item and returns shaped MemoryItem", async () => {
    const db = makeSingleDb({ data: memRow, error: null });
    const result = await recordMemory(db, "repo-1", "sess-1", {
      type: "decision",
      title: "Use optimistic locking",
      content: "Apply optimistic locking for invoice updates to avoid contention",
    });
    expect(result.id).toBe("mem-1");
    expect(result.type).toBe("decision");
    expect(result.status).toBe("open");
    expect(result.resolution).toBeNull();
    expect(result.closedAt).toBeNull();
  });

  it("throws on DB error", async () => {
    const db = makeSingleDb({ data: null, error: { message: "fail" } });
    await expect(
      recordMemory(db, "repo-1", "sess-1", { type: "decision", title: "t", content: "c" })
    ).rejects.toThrow("fail");
  });
});

describe("closeMemory", () => {
  it("updates item and returns closed MemoryItem", async () => {
    const closedRow = { ...memRow, status: "closed", resolution: "resolved via X", closed_at: "2026-03-13T01:00:00Z" };
    // First call: select to check current status (returns open item)
    // Second call: update to close it (returns closed item)
    let callCount = 0;
    const db = {
      from: vi.fn().mockImplementation(() => {
        callCount++;
        if (callCount === 1) {
          // check status
          return {
            select: vi.fn().mockReturnThis(),
            eq: vi.fn().mockReturnThis(),
            single: vi.fn().mockResolvedValue({ data: memRow, error: null }),
          };
        }
        // update
        return {
          update: vi.fn().mockReturnValue({
            eq: vi.fn().mockReturnThis(),
            select: vi.fn().mockReturnThis(),
            single: vi.fn().mockResolvedValue({ data: closedRow, error: null }),
          }),
        };
      }),
    } as any;
    const result = await closeMemory(db, "mem-1", "resolved via X");
    expect(result.status).toBe("closed");
    expect(result.resolution).toBe("resolved via X");
    expect(result.closedAt).toBe("2026-03-13T01:00:00Z");
  });

  it("throws if item not found (update returns no data)", async () => {
    let callCount = 0;
    const db = {
      from: vi.fn().mockImplementation(() => {
        callCount++;
        if (callCount === 1) {
          // select: item not found
          return {
            select: vi.fn().mockReturnThis(),
            eq: vi.fn().mockReturnThis(),
            single: vi.fn().mockResolvedValue({ data: null, error: null }),
          };
        }
        // update: also not found
        return {
          update: vi.fn().mockReturnValue({
            eq: vi.fn().mockReturnThis(),
            select: vi.fn().mockReturnThis(),
            single: vi.fn().mockResolvedValue({ data: null, error: { message: "no rows" } }),
          }),
        };
      }),
    } as any;
    await expect(closeMemory(db, "mem-1", "resolution")).rejects.toThrow();
  });

  it("throws if item already closed", async () => {
    const closedRow = { ...memRow, status: "closed" };
    const db = {
      from: vi.fn().mockReturnValue({
        select: vi.fn().mockReturnThis(),
        eq: vi.fn().mockReturnThis(),
        single: vi.fn().mockResolvedValue({ data: closedRow, error: null }),
      }),
    } as any;
    await expect(closeMemory(db, "mem-1", "resolution")).rejects.toThrow("already closed");
  });
});

describe("getMemoryItems", () => {
  it("returns shaped MemoryItems", async () => {
    const db = {
      from: vi.fn().mockReturnValue({
        select: vi.fn().mockReturnThis(),
        eq: vi.fn().mockReturnThis(),
        order: vi.fn().mockResolvedValue({ data: [memRow], error: null }),
      }),
    } as any;
    const result = await getMemoryItems(db, "repo-1");
    expect(result).toHaveLength(1);
    expect(result[0].id).toBe("mem-1");
  });

  it("returns [] on error (never throws)", async () => {
    const db = {
      from: vi.fn().mockReturnValue({
        select: vi.fn().mockReturnThis(),
        eq: vi.fn().mockReturnThis(),
        order: vi.fn().mockResolvedValue({ data: null, error: { message: "fail" } }),
      }),
    } as any;
    const result = await getMemoryItems(db, "repo-1");
    expect(result).toEqual([]);
  });
});
