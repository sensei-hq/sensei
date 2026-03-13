import { describe, it, expect, vi } from "vitest";
import { formatStats, stats } from "./stats.js";
import type { StatsResult } from "@sensei/collector";

// A minimal StatsResult for formatStats tests
function makeResult(overrides: Partial<StatsResult> = {}): StatsResult {
  return {
    period: { from: "2026-03-05", to: "2026-03-12" },
    total_calls: 2,
    tools: [
      { name: "search_index", calls: 1, success_rate: 1.0, avg_duration_ms: 150, last_called: Date.now() },
      { name: "Bash", calls: 1, success_rate: 0.0, avg_duration_ms: 300, last_called: Date.now() },
    ],
    sessions: 1,
    projects: 1,
    ...overrides,
  };
}

describe("formatStats", () => {
  it("default text output includes tool names and total calls", () => {
    const result = makeResult();
    const text = formatStats(result, { json: false });
    expect(text).toContain("search_index");
    expect(text).toContain("Bash");
    expect(text).toContain("2"); // total calls
  });

  it("--json output is valid JSON with expected keys", () => {
    const result = makeResult();
    const text = formatStats(result, { json: true });
    const parsed = JSON.parse(text) as Record<string, unknown>;
    expect(parsed.total_calls).toBeDefined();
    expect(Array.isArray(parsed.tools)).toBe(true);
    expect(parsed.period).toBeDefined();
  });

  it("tool-specific output includes tool name and success_rate", () => {
    const result = makeResult({
      tool: { name: "search_index", calls: 1, success_rate: 1.0, avg_duration_ms: 150, last_called: Date.now() },
    });
    const text = formatStats(result, { json: false });
    expect(text).toContain("search_index");
    expect(text).toMatch(/100%|1\.0/);
  });
});

describe("stats() with mock Supabase client", () => {
  it("logs error when Supabase client not configured", async () => {
    // Mock makeSenseiClient to return null (not configured)
    vi.mock("@sensei/shared", () => ({
      makeSenseiClient: vi.fn().mockResolvedValue(null),
    }));

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
