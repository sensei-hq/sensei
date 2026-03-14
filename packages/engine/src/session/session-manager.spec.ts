// packages/engine/src/session/session-manager.spec.ts
import { describe, it, expect, vi } from "vitest";
import { createSession, detectCrashedSessions, updateHeartbeat } from "./session-manager.js";

describe("createSession", () => {
  it("inserts a session row and returns id + createdAt", async () => {
    const db = {
      from: vi.fn().mockReturnValue({
        insert: vi.fn().mockReturnValue({
          select: vi.fn().mockReturnThis(),
          single: vi.fn().mockResolvedValue({
            data: { id: "sess-1", created_at: "2026-03-13T00:00:00Z" },
            error: null,
          }),
        }),
      }),
    } as any;
    const result = await createSession(db, "repo-1");
    expect(result.id).toBe("sess-1");
    expect(result.createdAt).toBe("2026-03-13T00:00:00Z");
    expect(db.from).toHaveBeenCalledWith("sessions");
  });

  it("throws on DB error", async () => {
    const db = {
      from: vi.fn().mockReturnValue({
        insert: vi.fn().mockReturnValue({
          select: vi.fn().mockReturnThis(),
          single: vi.fn().mockResolvedValue({ data: null, error: { message: "fail" } }),
        }),
      }),
    } as any;
    await expect(createSession(db, "repo-1")).rejects.toThrow("fail");
  });
});

describe("detectCrashedSessions", () => {
  it("returns [] when no idle sessions found", async () => {
    const db = {
      from: vi.fn().mockReturnValue({
        select: vi.fn().mockReturnThis(),
        eq: vi.fn().mockReturnThis(),
        lt: vi.fn().mockReturnThis(),
        order: vi.fn().mockReturnThis(),
        limit: vi.fn().mockResolvedValue({ data: [], error: null }),
      }),
    } as any;
    const result = await detectCrashedSessions(db, "repo-1");
    expect(result).toEqual([]);
  });

  it("returns [] on DB error (best-effort)", async () => {
    const db = {
      from: vi.fn().mockReturnValue({
        select: vi.fn().mockReturnThis(),
        eq: vi.fn().mockReturnThis(),
        lt: vi.fn().mockReturnThis(),
        order: vi.fn().mockReturnThis(),
        limit: vi.fn().mockResolvedValue({ data: null, error: { message: "db error" } }),
      }),
    } as any;
    const result = await detectCrashedSessions(db, "repo-1");
    expect(result).toEqual([]);
  });

  it("marks idle sessions as crashed and returns CrashedSession with lastHeartbeat", async () => {
    const idleSession = {
      id: "sess-old",
      created_at: "2026-03-13T00:00:00Z",
      last_heartbeat: "2026-03-13T00:00:00Z",
    };
    const snapshotRow = {
      id: "snap-1", kind: "manual",
      progress_summary: "mid-task", next_step_hint: null,
      completed_steps: [], in_flight_files: [], worktree_refs: [],
      diff_stat_summary: null, created_at: "2026-03-13T00:01:00Z",
    };
    // Mock: first from("sessions") returns idle session, second from("sessions") marks crashed,
    // then from("snapshots") returns the snapshot
    let callCount = 0;
    const db = {
      from: vi.fn().mockImplementation((table: string) => {
        if (table === "sessions" && callCount === 0) {
          callCount++;
          return {
            select: vi.fn().mockReturnThis(),
            eq: vi.fn().mockReturnThis(),
            lt: vi.fn().mockReturnThis(),
            order: vi.fn().mockReturnThis(),
            limit: vi.fn().mockResolvedValue({ data: [idleSession], error: null }),
          };
        }
        if (table === "sessions" && callCount === 1) {
          callCount++;
          return {
            update: vi.fn().mockReturnValue({
              in: vi.fn().mockResolvedValue({ error: null }),
            }),
          };
        }
        // snapshots table
        return {
          select: vi.fn().mockReturnThis(),
          eq: vi.fn().mockReturnThis(),
          order: vi.fn().mockReturnThis(),
          limit: vi.fn().mockReturnValue({
            single: vi.fn().mockResolvedValue({ data: snapshotRow, error: null }),
          }),
        };
      }),
    } as any;

    const result = await detectCrashedSessions(db, "repo-1");
    expect(result).toHaveLength(1);
    expect(result[0].id).toBe("sess-old");
    expect(result[0].lastHeartbeat).toBe("2026-03-13T00:00:00Z");
    expect(result[0].latestSnapshot?.id).toBe("snap-1");
  });
});

describe("updateHeartbeat", () => {
  it("never throws, even on DB error", async () => {
    const db = {
      from: vi.fn().mockReturnValue({
        update: vi.fn().mockReturnValue({
          eq: vi.fn().mockResolvedValue({ error: { message: "fail" } }),
        }),
      }),
    } as any;
    await expect(updateHeartbeat(db, "sess-1")).resolves.toBeUndefined();
  });
});
