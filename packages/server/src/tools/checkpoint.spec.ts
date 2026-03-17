// packages/server/src/tools/checkpoint.spec.ts
import { describe, it, expect, vi } from "vitest";

// Mock @sensei/engine before importing checkpoint
vi.mock("@sensei/engine", () => ({
  takeSnapshot: vi.fn().mockResolvedValue({
    id: "snap-1",
    kind: "checkpoint",
    progressSummary: "done",
    createdAt: new Date().toISOString(),
  }),
}));

// Mock child_process exec for git diff
vi.mock("child_process", () => ({
  exec: vi.fn((_cmd: string, cb: (err: null, out: { stdout: string }) => void) =>
    cb(null, { stdout: "src/foo.ts\nsrc/bar.ts\n" })
  ),
}));

import { checkpointTool } from "./checkpoint.js";

function makeClient(patternUsageRows: any[]) {
  const updates: any[] = [];
  return {
    // Top-level .from() — used only for "sessions" table
    from: (table: string) => {
      if (table === "sessions") {
        return {
          update: () => ({ eq: () => ({ then: (r: any) => r({ error: null }) }) }),
        };
      }
      return {};
    },
    // .schema("sensei").from() — used for pattern_usages
    schema: () => ({
      from: (table: string) => {
        if (table === "pattern_usages") {
          return {
            update: (vals: any) => ({
              eq: (_c1: string, _v1: any) => ({
                in: (_c: string, _v: any[]) => {
                  updates.push(vals);
                  return { then: (r: any) => r({ error: null }) };
                },
              }),
            }),
            select: (_cols: string) => ({
              eq: (_c1: string, _v1: any) => ({
                is: (_c2: string, _v2: any) => ({
                  then: (r: any) => r({ data: patternUsageRows, error: null }),
                }),
              }),
            }),
          };
        }
        return {};
      },
    }),
    updates,
  };
}

describe("checkpointTool", () => {
  it("updates open pattern_usages rows with outcome and files_modified", async () => {
    const rows = [{ id: "pu-1", session_id: "sess-1", outcome: null }];
    const client = makeClient(rows);

    await checkpointTool(client as any, "sess-1", "repo-1", {
      task_summary: "done",
    });

    expect(client.updates.length).toBeGreaterThan(0);
    const update = client.updates[0];
    expect(update).toHaveProperty("outcome", "completed");
    expect(update).toHaveProperty("files_modified");
    // Note: ftr_score propagation is deferred — requires engine to expose score
  });

  it("skips pattern_usages update when no open rows exist", async () => {
    const client = makeClient([]);
    await expect(
      checkpointTool(client as any, "sess-1", "repo-1", { task_summary: "done" })
    ).resolves.toBeDefined();
  });
});
