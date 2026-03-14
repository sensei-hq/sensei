import { describe, it, expect, vi } from "vitest";
import { computeFtr, computeAndStoreFtr } from "./ftr.js";
import type { FtrSignals } from "./ftr.js";

const perfect: FtrSignals = {
  snapshotCount: 1,
  toolErrorRate: 0,
  completedCleanly: true,
  hasDescription: true,
};

describe("computeFtr", () => {
  it("returns 1.0 for perfect signals", () => {
    expect(computeFtr(perfect)).toBe(1.0);
  });

  it("deducts 0.05 per extra snapshot beyond the first", () => {
    expect(computeFtr({ ...perfect, snapshotCount: 2 })).toBe(0.95);
    expect(computeFtr({ ...perfect, snapshotCount: 3 })).toBe(0.9);
    expect(computeFtr({ ...perfect, snapshotCount: 4 })).toBe(0.85);
  });

  it("caps snapshot penalty at -0.30 (7+ snapshots)", () => {
    expect(computeFtr({ ...perfect, snapshotCount: 10 })).toBe(0.7);
    expect(computeFtr({ ...perfect, snapshotCount: 100 })).toBe(0.7);
  });

  it("deducts 0.10 for error rate in [0.10, 0.20)", () => {
    expect(computeFtr({ ...perfect, toolErrorRate: 0.10 })).toBe(0.9);
    expect(computeFtr({ ...perfect, toolErrorRate: 0.19 })).toBe(0.9);
  });

  it("deducts 0.20 for error rate >= 0.20", () => {
    expect(computeFtr({ ...perfect, toolErrorRate: 0.20 })).toBe(0.8);
    expect(computeFtr({ ...perfect, toolErrorRate: 1.0 })).toBe(0.8);
  });

  it("deducts 0.30 when session did not complete cleanly", () => {
    expect(computeFtr({ ...perfect, completedCleanly: false })).toBe(0.7);
  });

  it("caps score at 0.70 when hasDescription is false", () => {
    expect(computeFtr({ ...perfect, hasDescription: false })).toBe(0.7);
    // Even if all other signals are perfect, cap applies
    expect(computeFtr({ ...perfect, hasDescription: false, snapshotCount: 2 })).toBe(0.7);
  });

  it("maximum penalties stack to yield 0.20 (the mathematical floor)", () => {
    // Maximum possible penalty: -0.30 (snap cap) + -0.20 (error) + -0.30 (crash) = -0.80
    // 1.0 - 0.80 = 0.20 is the lowest reachable score before the no-description cap
    const worst: FtrSignals = {
      snapshotCount: 10,       // -0.30 (capped)
      toolErrorRate: 0.5,      // -0.20
      completedCleanly: false, // -0.30
      hasDescription: true,
    };
    expect(computeFtr(worst)).toBe(0.2);
  });

  it("hasDescription=false cap applies after other penalties", () => {
    // 1.0 - 0.05 (1 extra snap) = 0.95, then capped at 0.70
    expect(computeFtr({ ...perfect, snapshotCount: 2, hasDescription: false })).toBe(0.7);
    // 1.0 - 0.30 (no clean) = 0.70, then capped at min(0.70, 0.70) = 0.70
    expect(computeFtr({ ...perfect, completedCleanly: false, hasDescription: false })).toBe(0.7);
    // 1.0 - 0.30 - 0.30 = 0.40, then capped at min(0.40, 0.70) = 0.40
    expect(computeFtr({ ...perfect, snapshotCount: 10, completedCleanly: false, hasDescription: false })).toBe(0.4);
  });
});

describe("computeAndStoreFtr", () => {
  function makeDb(overrides: {
    snapshotCount?: number;
    turns?: Array<{ success: boolean | null }>;
    sessionStatus?: string;
    taskDescription?: string | null;
    updateError?: string | null;
  } = {}) {
    const {
      snapshotCount = 1,
      turns = [],
      sessionStatus = "completed",
      taskDescription = "add new feature",
      updateError = null,
    } = overrides;

    // Disambiguate the two task_sessions calls by their own counter
    let taskSessionsCallCount = 0;
    return {
      from: vi.fn().mockImplementation((table: string) => {
        if (table === "snapshots") {
          return {
            select: vi.fn().mockReturnThis(),
            eq: vi.fn().mockResolvedValue({ count: snapshotCount, error: null }),
          };
        }
        if (table === "task_turns") {
          return {
            select: vi.fn().mockReturnThis(),
            eq: vi.fn().mockResolvedValue({ data: turns, error: null }),
          };
        }
        if (table === "sessions") {
          return {
            select: vi.fn().mockReturnThis(),
            eq: vi.fn().mockReturnThis(),
            single: vi.fn().mockResolvedValue({ data: { status: sessionStatus }, error: null }),
          };
        }
        // task_sessions: first call = select task_description, second = update ftr_score
        taskSessionsCallCount++;
        if (taskSessionsCallCount === 1) {
          return {
            select: vi.fn().mockReturnThis(),
            eq: vi.fn().mockReturnThis(),
            single: vi.fn().mockResolvedValue({ data: { task_description: taskDescription }, error: null }),
          };
        }
        return {
          update: vi.fn().mockReturnValue({
            eq: vi.fn().mockResolvedValue({ error: updateError ? { message: updateError } : null }),
          }),
        };
      }),
    } as any;
  }

  it("computes FTR from DB signals and stores result", async () => {
    const db = makeDb({ snapshotCount: 2, turns: [{ success: true }, { success: true }] });
    const result = await computeAndStoreFtr(db, "ts-1", "sess-1");
    // snapshotCount=2 → -0.05, no errors, clean, hasDescription → 0.95
    expect(result.score).toBe(0.95);
    expect(result.signals.snapshotCount).toBe(2);
    expect(result.signals.toolErrorRate).toBe(0);
    expect(result.signals.completedCleanly).toBe(true);
    expect(result.signals.hasDescription).toBe(true);
  });

  it("sets hasDescription=false when task_description is null", async () => {
    const db = makeDb({ taskDescription: null });
    const result = await computeAndStoreFtr(db, "ts-1", "sess-1");
    expect(result.signals.hasDescription).toBe(false);
    expect(result.score).toBeLessThanOrEqual(0.7);
  });

  it("sets completedCleanly=false when session status is not completed", async () => {
    const db = makeDb({ sessionStatus: "crashed" });
    const result = await computeAndStoreFtr(db, "ts-1", "sess-1");
    expect(result.signals.completedCleanly).toBe(false);
    expect(result.score).toBeLessThanOrEqual(0.7);
  });

  it("throws on DB update error", async () => {
    const db = makeDb({ updateError: "write failed" });
    await expect(computeAndStoreFtr(db, "ts-1", "sess-1")).rejects.toThrow("write failed");
  });

  it("zero-turn session gets toolErrorRate=0 (best-case default)", async () => {
    const db = makeDb({ turns: [] });
    const result = await computeAndStoreFtr(db, "ts-1", "sess-1");
    expect(result.signals.toolErrorRate).toBe(0);
    // Perfect signals (snapshotCount=1, no errors, clean, hasDescription) → 1.0
    expect(result.score).toBe(1.0);
  });

  it("throws on DB read error (snapshots)", async () => {
    const db = {
      from: vi.fn().mockImplementation((table: string) => {
        if (table === "snapshots") {
          return { select: vi.fn().mockReturnThis(), eq: vi.fn().mockResolvedValue({ count: null, error: { message: "snapshots unavailable" } }) };
        }
        // Other tables never reached
        return { select: vi.fn().mockReturnThis(), eq: vi.fn().mockReturnThis(), single: vi.fn().mockResolvedValue({ data: null, error: null }) };
      }),
    } as any;
    await expect(computeAndStoreFtr(db, "ts-1", "sess-1")).rejects.toThrow("snapshots unavailable");
  });
});
