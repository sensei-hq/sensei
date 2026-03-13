import { describe, it, expect, vi, beforeEach } from "vitest";
import { writeEventToSupabase } from "./supabase-writer.js";

const mockInsert = vi.fn().mockReturnValue({ error: null });
const mockFrom = vi.fn(() => ({ insert: mockInsert }));
const mockClient = { from: mockFrom } as any;

describe("writeEventToSupabase", () => {
  beforeEach(() => vi.clearAllMocks());

  it("inserts event with correct schema and fields", async () => {
    await writeEventToSupabase(mockClient, {
      user_uuid: "user-1",
      session_id: "sess-1",
      repo_id: "repo-uuid",
      phase: "post",
      tool: "Edit",
      project_path: "/projects/foo",
      input: { file: "src/a.ts" },
      ts: new Date("2026-01-01T00:00:00Z"),
    });

    expect(mockFrom).toHaveBeenCalledWith("events");
    const inserted = mockInsert.mock.calls[0][0];
    expect(inserted.tool).toBe("Edit");
    expect(inserted.phase).toBe("post");
    expect(inserted.user_uuid).toBe("user-1");
  });

  it("does not throw when insert returns error", async () => {
    mockInsert.mockReturnValueOnce({ error: new Error("network error") });
    await expect(writeEventToSupabase(mockClient, {
      user_uuid: "u", session_id: "s", repo_id: null,
      phase: "pre", tool: "Bash", project_path: "/p",
      input: null, ts: new Date(),
    })).resolves.toBeUndefined();
  });
});
