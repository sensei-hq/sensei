// packages/engine/src/analytics/task-session.spec.ts
import { describe, it, expect, vi } from "vitest";
import { createTaskSession, recordTaskTurn } from "./task-session.js";

function makeInsertDb(result: { data: Record<string, unknown> | null; error: { message: string } | null }) {
  return {
    from: vi.fn().mockReturnValue({
      insert: vi.fn().mockReturnValue({
        select: vi.fn().mockReturnThis(),
        single: vi.fn().mockResolvedValue(result),
      }),
    }),
  } as any;
}

const taskRow = {
  id: "ts-1",
  session_id: "sess-1",
  repo_id: "repo-1",
  task_description: "fix the broken auth middleware",
  task_type: "fix",
  status: "in_progress",
  ftr_score: null,
  ftr_signals: null,
  created_at: "2026-03-13T00:00:00Z",
  completed_at: null,
};

describe("createTaskSession", () => {
  it("inserts a task_session row and returns TaskSessionInfo", async () => {
    const db = makeInsertDb({ data: taskRow, error: null });
    const result = await createTaskSession(db, "sess-1", "repo-1", "fix the broken auth middleware");
    const insertArg = (db.from("task_sessions").insert as ReturnType<typeof vi.fn>).mock.calls[0][0];
    expect(insertArg.task_type).toBe("fix");
    expect(result.id).toBe("ts-1");
    expect(result.taskType).toBe("fix");
    expect(result.createdAt).toBe("2026-03-13T00:00:00Z");
    expect(db.from).toHaveBeenCalledWith("task_sessions");
  });

  it("detects task_type=feat for 'add new feature'", async () => {
    const db = makeInsertDb({ data: { ...taskRow, task_type: "feat" }, error: null });
    await createTaskSession(db, "sess-1", "repo-1", "add new feature");
    const insertArg = (db.from("task_sessions").insert as ReturnType<typeof vi.fn>).mock.calls[0][0];
    expect(insertArg.task_type).toBe("feat");
  });

  it("detects task_type=refactor for 'refactor the service layer'", async () => {
    const db = makeInsertDb({ data: { ...taskRow, task_type: "refactor" }, error: null });
    await createTaskSession(db, "sess-1", "repo-1", "refactor the service layer");
    const insertArg = (db.from("task_sessions").insert as ReturnType<typeof vi.fn>).mock.calls[0][0];
    expect(insertArg.task_type).toBe("refactor");
  });

  it("detects task_type=docs for 'document the API'", async () => {
    const db = makeInsertDb({ data: { ...taskRow, task_type: "docs" }, error: null });
    await createTaskSession(db, "sess-1", "repo-1", "document the API");
    const insertArg = (db.from("task_sessions").insert as ReturnType<typeof vi.fn>).mock.calls[0][0];
    expect(insertArg.task_type).toBe("docs");
  });

  it("detects task_type=test for 'add spec coverage'", async () => {
    const db = makeInsertDb({ data: { ...taskRow, task_type: "test" }, error: null });
    await createTaskSession(db, "sess-1", "repo-1", "add spec coverage");
    const insertArg = (db.from("task_sessions").insert as ReturnType<typeof vi.fn>).mock.calls[0][0];
    expect(insertArg.task_type).toBe("test");
  });

  it("detects task_type=chore for 'bump dependencies'", async () => {
    const db = makeInsertDb({ data: { ...taskRow, task_type: "chore" }, error: null });
    await createTaskSession(db, "sess-1", "repo-1", "bump dependencies");
    const insertArg = (db.from("task_sessions").insert as ReturnType<typeof vi.fn>).mock.calls[0][0];
    expect(insertArg.task_type).toBe("chore");
  });

  it("defaults to task_type=unknown when no description matches", async () => {
    const db = makeInsertDb({ data: { ...taskRow, task_type: "unknown" }, error: null });
    await createTaskSession(db, "sess-1", "repo-1", "xyz 123 unrecognised");
    const insertArg = (db.from("task_sessions").insert as ReturnType<typeof vi.fn>).mock.calls[0][0];
    expect(insertArg.task_type).toBe("unknown");
  });

  it("uses task_type=unknown and null description when no description provided", async () => {
    const db = makeInsertDb({ data: { ...taskRow, task_description: null, task_type: "unknown" }, error: null });
    await createTaskSession(db, "sess-1", "repo-1");
    const insertArg = (db.from("task_sessions").insert as ReturnType<typeof vi.fn>).mock.calls[0][0];
    expect(insertArg.task_description).toBeNull();
    expect(insertArg.task_type).toBe("unknown");
  });

  it("throws on DB error", async () => {
    const db = makeInsertDb({ data: null, error: { message: "fail" } });
    await expect(createTaskSession(db, "sess-1", "repo-1", "fix bug")).rejects.toThrow("fail");
  });
});

describe("recordTaskTurn", () => {
  it("inserts a task_turns row", async () => {
    const db = {
      from: vi.fn().mockReturnValue({
        insert: vi.fn().mockResolvedValue({ error: null }),
      }),
    } as any;
    await recordTaskTurn(db, "ts-1", "repo-1", "Bash", true);
    expect(db.from).toHaveBeenCalledWith("task_turns");
    const insertArg = (db.from("task_turns").insert as ReturnType<typeof vi.fn>).mock.calls[0][0];
    expect(insertArg.task_session_id).toBe("ts-1");
    expect(insertArg.repo_id).toBe("repo-1");
    expect(insertArg.tool).toBe("Bash");
  });

  it("never throws on DB error (silent best-effort)", async () => {
    const db = {
      from: vi.fn().mockReturnValue({
        insert: vi.fn().mockRejectedValue(new Error("db down")),
      }),
    } as any;
    await expect(recordTaskTurn(db, "ts-1", "repo-1", "Bash", false)).resolves.toBeUndefined();
  });
});
