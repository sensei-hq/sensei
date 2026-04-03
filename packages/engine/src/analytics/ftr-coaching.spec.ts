import { describe, it, expect } from "vitest";
import { detectFtrCoaching } from "./ftr-coaching.js";

function makeClient(rows: Array<{ ftr_score: number; ftr_signals: Record<string, unknown> }>) {
  return {
    from: () => ({
      select: () => ({
        eq: () => ({
          eq: () => ({
            not: () => ({
              order: () => ({
                limit: () => Promise.resolve({ data: rows, error: null }),
              }),
            }),
          }),
        }),
      }),
    }),
  } as any;
}

describe("detectFtrCoaching", () => {
  it("returns empty array when fewer than 2 completed sessions", async () => {
    const client = makeClient([{ ftr_score: 0.5, ftr_signals: { snapshotCount: 5, toolErrorRate: 0.2, completedCleanly: false, hasDescription: false } }]);
    const hints = await detectFtrCoaching(client, "repo-1");
    expect(hints).toEqual([]);
  });

  it("returns empty array when all recent FTR scores are high", async () => {
    const goodSignals = { snapshotCount: 1, toolErrorRate: 0, completedCleanly: true, hasDescription: true };
    const rows = Array(5).fill({ ftr_score: 0.9, ftr_signals: goodSignals });
    const client = makeClient(rows);
    const hints = await detectFtrCoaching(client, "repo-1");
    expect(hints).toEqual([]);
  });

  it("detects high snapshot count pattern", async () => {
    const signals = { snapshotCount: 5, toolErrorRate: 0, completedCleanly: true, hasDescription: true };
    const rows = Array(4).fill({ ftr_score: 0.6, ftr_signals: signals });
    const client = makeClient(rows);
    const hints = await detectFtrCoaching(client, "repo-1");
    const h = hints.find(h => h.pattern === "high_snapshot_count");
    expect(h).toBeDefined();
    expect(h!.frequency).toBe(4);
  });

  it("detects high tool error rate pattern", async () => {
    const signals = { snapshotCount: 1, toolErrorRate: 0.15, completedCleanly: true, hasDescription: true };
    const rows = Array(3).fill({ ftr_score: 0.6, ftr_signals: signals });
    const client = makeClient(rows);
    const hints = await detectFtrCoaching(client, "repo-1");
    const h = hints.find(h => h.pattern === "high_tool_error_rate");
    expect(h).toBeDefined();
    expect(h!.frequency).toBe(3);
  });

  it("detects incomplete sessions pattern", async () => {
    const signals = { snapshotCount: 1, toolErrorRate: 0, completedCleanly: false, hasDescription: true };
    const rows = Array(3).fill({ ftr_score: 0.5, ftr_signals: signals });
    const client = makeClient(rows);
    const hints = await detectFtrCoaching(client, "repo-1");
    const h = hints.find(h => h.pattern === "incomplete_sessions");
    expect(h).toBeDefined();
  });

  it("detects missing descriptions pattern", async () => {
    const signals = { snapshotCount: 1, toolErrorRate: 0, completedCleanly: true, hasDescription: false };
    const rows = Array(3).fill({ ftr_score: 0.65, ftr_signals: signals });
    const client = makeClient(rows);
    const hints = await detectFtrCoaching(client, "repo-1");
    const h = hints.find(h => h.pattern === "missing_descriptions");
    expect(h).toBeDefined();
  });

  it("returns empty array on DB error", async () => {
    const client = {
      from: () => ({ select: () => ({ eq: () => ({ eq: () => ({ not: () => ({ order: () => ({ limit: () => Promise.resolve({ data: null, error: new Error("DB error") }) }) }) }) }) }) }),
    } as any;
    const hints = await detectFtrCoaching(client, "repo-1");
    expect(hints).toEqual([]);
  });
});
