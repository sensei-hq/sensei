import { describe, it, expect, vi } from "vitest";
import { recordPatternUse } from "./record-pattern-use.js";

vi.mock("../activity-log.js", () => ({
  getActivityLog: () => ({
    logPatternUse: vi.fn(),
  }),
}));

describe("recordPatternUse", () => {
  it("returns confirmation string", async () => {
    const result = await recordPatternUse("repo-1", "session-1", "mcp-tool");
    expect(result).toContain("mcp-tool");
  });

  it("works without a session_id", async () => {
    const result = await recordPatternUse("repo-1", null, "adapter");
    expect(result).toContain("adapter");
  });

  it("succeeds when activity log records the pattern", async () => {
    const result = await recordPatternUse("repo-1", null, "adapter");
    expect(typeof result).toBe("string");
  });
});
