// packages/server/src/tools/checkpoint.spec.ts
import { describe, it, expect, vi } from "vitest";

// Mock activity-log to avoid loading better-sqlite3 native bindings in test env
vi.mock("../activity-log.js", () => ({
  getActivityLog: vi.fn(() => ({
    updateSession: vi.fn(),
    logSnapshot: vi.fn().mockReturnValue("snap-local-1"),
  })),
}));

import { checkpointTool } from "./checkpoint.js";

describe("checkpointTool", () => {
  it("returns a checkpoint result with local snapshot ID", async () => {
    const result = await checkpointTool("sess-1", "repo-1", { task_summary: "done" });
    expect(result).toHaveProperty("id", "snap-local-1");
    expect(result).toHaveProperty("kind", "checkpoint");
    expect(result).toHaveProperty("progressSummary", "done");
  });

  it("returns createdAt timestamp", async () => {
    const result = await checkpointTool("sess-1", "repo-1", { task_summary: "done" });
    expect(typeof result.createdAt).toBe("string");
    expect(new Date(result.createdAt).getTime()).toBeGreaterThan(0);
  });

  it("resolves cleanly when localSessionId is not provided", async () => {
    await expect(
      checkpointTool("sess-1", "repo-1", { task_summary: "done" })
    ).resolves.toBeDefined();
  });

  it("accepts optional repoPath and localSessionId", async () => {
    await expect(
      checkpointTool("sess-1", "repo-1", { task_summary: "step done", completed_steps: ["a", "b"] }, "/repo", "local-sess-1")
    ).resolves.toHaveProperty("id", "snap-local-1");
  });
});
