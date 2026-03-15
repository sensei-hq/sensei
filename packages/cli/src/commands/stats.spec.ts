// packages/cli/src/commands/stats.spec.ts
import { describe, it, expect, vi } from "vitest";
import { formatStats, buildStatsResult } from "./stats.js";
import type { StatsResult } from "./stats.js";

function makeResult(overrides: Partial<StatsResult> = {}): StatsResult {
  return {
    period: { from: "2026-03-05", to: "2026-03-12" },
    sessions: { total: 5, completed: 3, abandoned: 1, inProgress: 1 },
    avgFtr: 0.82,
    topTools: [
      { name: "Bash", calls: 42, successRate: 0.95, avgDurationMs: 200 },
      { name: "Read", calls: 28, successRate: 1.0, avgDurationMs: 50 },
    ],
    errorCount: 3,
    errorSessions: 2,
    ...overrides,
  };
}

describe("formatStats", () => {
  it("default text output includes session counts and avg FTR", () => {
    const text = formatStats(makeResult(), { json: false });
    expect(text).toContain("5");        // total sessions
    expect(text).toContain("0.82");     // avgFtr
    expect(text).toContain("Bash");     // top tool
  });

  it("shows abandoned count in session breakdown", () => {
    const text = formatStats(makeResult(), { json: false });
    expect(text).toContain("abandoned");
  });

  it("--json output is valid JSON with expected keys", () => {
    const text = formatStats(makeResult(), { json: true });
    const parsed = JSON.parse(text) as StatsResult;
    expect(parsed.sessions.total).toBe(5);
    expect(parsed.avgFtr).toBe(0.82);
    expect(Array.isArray(parsed.topTools)).toBe(true);
  });

  it("shows '—' for avgFtr when null (no completed sessions)", () => {
    const text = formatStats(makeResult({ avgFtr: null }), { json: false });
    expect(text).toContain("—");
  });
});

describe("buildStatsResult", () => {
  it("aggregates task_sessions and task_turns into StatsResult", () => {
    const sessions = [
      { status: "completed", ftr_score: 0.9, id: "ts-1" },
      { status: "completed", ftr_score: 0.8, id: "ts-2" },
      { status: "abandoned", ftr_score: null, id: "ts-3" },
    ];
    const turns = [
      { tool: "Bash", success: true, duration_ms: 200, task_session_id: "ts-1" },
      { tool: "Bash", success: false, duration_ms: 100, task_session_id: "ts-1" },
      { tool: "Read", success: true, duration_ms: 50, task_session_id: "ts-2" },
    ];
    const result = buildStatsResult(sessions as any, turns as any, { from: "2026-03-05", to: "2026-03-12" });

    expect(result.sessions.total).toBe(3);
    expect(result.sessions.completed).toBe(2);
    expect(result.sessions.abandoned).toBe(1);
    expect(result.sessions.inProgress).toBe(0);
    expect(result.avgFtr).toBeCloseTo(0.85);
    expect(result.topTools[0].name).toBe("Bash");
    expect(result.topTools[0].calls).toBe(2);
    expect(result.topTools[0].successRate).toBeCloseTo(0.5);
    expect(result.errorCount).toBe(1);
    expect(result.errorSessions).toBe(1);  // ts-1 had one error turn
  });
});

describe("stats() integration", () => {
  it("logs error when Supabase client not configured", async () => {
    vi.mock("@sensei/shared", () => ({
      makeSenseiClient: vi.fn().mockResolvedValue(null),
    }));

    const { stats } = await import("./stats.js");
    const output: string[] = [];
    const origError = console.error;
    console.error = (...args: unknown[]) => output.push(args.join(" "));
    try {
      await stats({ _repoPath: "/nonexistent" });
    } finally {
      console.error = origError;
    }
    expect(output.some(l => l.includes("not configured"))).toBe(true);
    vi.restoreAllMocks();
  });
});
