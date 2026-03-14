// packages/engine/src/session/snapshot.spec.ts
import { describe, it, expect, vi } from "vitest";
import { takeSnapshot, getLatestSnapshot } from "./snapshot.js";

const baseOpts = {
  kind: "manual" as const,
  progressSummary: "Working on auth module",
};

function makeDb(insertResult: any, selectResult: any) {
  const singleInsert = { select: vi.fn().mockReturnThis(), single: vi.fn().mockResolvedValue(insertResult) };
  const singleSelect = { single: vi.fn().mockResolvedValue(selectResult) };
  return {
    from: vi.fn().mockReturnValue({
      insert: vi.fn().mockReturnValue(singleInsert),
      select: vi.fn().mockReturnThis(),
      eq: vi.fn().mockReturnThis(),
      order: vi.fn().mockReturnThis(),
      limit: vi.fn().mockReturnValue(singleSelect),
    }),
  } as any;
}

const snapshotRow = {
  id: "snap-1",
  kind: "manual",
  progress_summary: "Working on auth module",
  next_step_hint: null,
  completed_steps: [],
  in_flight_files: [],
  worktree_refs: [],
  diff_stat_summary: null,
  created_at: "2026-03-13T00:00:00Z",
};

describe("takeSnapshot", () => {
  it("inserts snapshot and returns shaped Snapshot", async () => {
    const db = makeDb({ data: snapshotRow, error: null }, null);
    const result = await takeSnapshot(db, "sess-1", "repo-1", baseOpts);
    expect(result.id).toBe("snap-1");
    expect(result.kind).toBe("manual");
    expect(result.progressSummary).toBe("Working on auth module");
    expect(result.nextStepHint).toBeNull();
    expect(result.completedSteps).toEqual([]);
    expect(result.createdAt).toBe("2026-03-13T00:00:00Z");
  });

  it("throws on DB error", async () => {
    const db = makeDb({ data: null, error: { message: "fail" } }, null);
    await expect(takeSnapshot(db, "sess-1", "repo-1", baseOpts)).rejects.toThrow("fail");
  });
});

describe("getLatestSnapshot", () => {
  it("returns null when no snapshots exist", async () => {
    const db = {
      from: vi.fn().mockReturnValue({
        select: vi.fn().mockReturnThis(),
        eq: vi.fn().mockReturnThis(),
        order: vi.fn().mockReturnThis(),
        limit: vi.fn().mockReturnValue({ single: vi.fn().mockResolvedValue({ data: null, error: null }) }),
      }),
    } as any;
    const result = await getLatestSnapshot(db, "sess-1");
    expect(result).toBeNull();
  });

  it("returns shaped Snapshot when found", async () => {
    const db = {
      from: vi.fn().mockReturnValue({
        select: vi.fn().mockReturnThis(),
        eq: vi.fn().mockReturnThis(),
        order: vi.fn().mockReturnThis(),
        limit: vi.fn().mockReturnValue({ single: vi.fn().mockResolvedValue({ data: snapshotRow, error: null }) }),
      }),
    } as any;
    const result = await getLatestSnapshot(db, "sess-1");
    expect(result?.id).toBe("snap-1");
    expect(result?.progressSummary).toBe("Working on auth module");
  });
});
