// packages/engine/src/analytics/task-session.spec.ts
import { describe, it, expect, vi } from "vitest";
import { createTaskSession, recordTaskTurn, completeTaskSession, getTaskSessions } from "./task-session.js";

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

describe("completeTaskSession", () => {
  it("calls computeAndStoreFtr and marks session completed", async () => {
    // completeTaskSession internally calls computeAndStoreFtr which does 5 DB calls,
    // then does a final update on task_sessions. We mock all DB calls.
    let callCount = 0;
    let statusUpdateArg: Record<string, unknown> | undefined;
    const db = {
      from: vi.fn().mockImplementation((table: string) => {
        callCount++;
        if (table === "snapshots") {
          return { select: vi.fn().mockReturnThis(), eq: vi.fn().mockResolvedValue({ count: 1, error: null }) };
        }
        if (table === "task_turns") {
          return { select: vi.fn().mockReturnThis(), eq: vi.fn().mockResolvedValue({ data: [], error: null }) };
        }
        if (table === "sessions") {
          return { select: vi.fn().mockReturnThis(), eq: vi.fn().mockReturnThis(), single: vi.fn().mockResolvedValue({ data: { status: "completed" }, error: null }) };
        }
        if (callCount === 4) {
          // task_sessions select task_description
          return { select: vi.fn().mockReturnThis(), eq: vi.fn().mockReturnThis(), single: vi.fn().mockResolvedValue({ data: { task_description: "add feature" }, error: null }) };
        }
        if (callCount === 5) {
          // task_sessions update ftr_score (from computeAndStoreFtr)
          return { update: vi.fn().mockReturnValue({ eq: vi.fn().mockResolvedValue({ error: null }) }) };
        }
        // task_sessions update status='completed' (final call from completeTaskSession)
        return { update: vi.fn().mockImplementation((arg: Record<string, unknown>) => {
          statusUpdateArg = arg;
          return { eq: vi.fn().mockResolvedValue({ error: null }) };
        }) };
      }),
    } as any;

    const result = await completeTaskSession(db, "ts-1", "sess-1");
    expect(result.score).toBe(1.0);  // perfect signals: count=1, no errors, clean, hasDescription
    expect(result.signals.completedCleanly).toBe(true);
    expect(statusUpdateArg?.status).toBe("completed");
    expect(statusUpdateArg?.completed_at).toBeDefined();
  });

  it("throws if status update fails", async () => {
    let callCount = 0;
    const db = {
      from: vi.fn().mockImplementation((table: string) => {
        callCount++;
        if (table === "snapshots") return { select: vi.fn().mockReturnThis(), eq: vi.fn().mockResolvedValue({ count: 1, error: null }) };
        if (table === "task_turns") return { select: vi.fn().mockReturnThis(), eq: vi.fn().mockResolvedValue({ data: [], error: null }) };
        if (table === "sessions") return { select: vi.fn().mockReturnThis(), eq: vi.fn().mockReturnThis(), single: vi.fn().mockResolvedValue({ data: { status: "completed" }, error: null }) };
        if (callCount === 4) return { select: vi.fn().mockReturnThis(), eq: vi.fn().mockReturnThis(), single: vi.fn().mockResolvedValue({ data: { task_description: "fix" }, error: null }) };
        if (callCount === 5) return { update: vi.fn().mockReturnValue({ eq: vi.fn().mockResolvedValue({ error: null }) }) };
        return { update: vi.fn().mockReturnValue({ eq: vi.fn().mockResolvedValue({ error: { message: "update failed" } }) }) };
      }),
    } as any;

    await expect(completeTaskSession(db, "ts-1", "sess-1")).rejects.toThrow("update failed");
  });
});

describe("getTaskSessions", () => {
  it("returns shaped TaskSession array", async () => {
    const db = {
      from: vi.fn().mockReturnValue({
        select: vi.fn().mockReturnThis(),
        eq: vi.fn().mockReturnThis(),
        gte: vi.fn().mockReturnThis(),
        order: vi.fn().mockResolvedValue({
          data: [{
            id: "ts-1",
            session_id: "sess-1",
            repo_id: "repo-1",
            task_description: "fix bug",
            task_type: "fix",
            status: "completed",
            ftr_score: 0.95,
            ftr_signals: { snapshotCount: 2, toolErrorRate: 0, completedCleanly: true, hasDescription: true },
            created_at: "2026-03-13T00:00:00Z",
            completed_at: "2026-03-13T01:00:00Z",
          }],
          error: null,
        }),
      }),
    } as any;

    const result = await getTaskSessions(db, "repo-1");
    expect(result).toHaveLength(1);
    expect(result[0].id).toBe("ts-1");
    expect(result[0].taskType).toBe("fix");
    expect(result[0].status).toBe("completed");
    expect(result[0].ftrScore).toBe(0.95);
    expect(result[0].ftrSignals).toEqual({ snapshotCount: 2, toolErrorRate: 0, completedCleanly: true, hasDescription: true });
  });

  it("returns [] on DB error (never throws)", async () => {
    const db = {
      from: vi.fn().mockReturnValue({
        select: vi.fn().mockReturnThis(),
        eq: vi.fn().mockReturnThis(),
        gte: vi.fn().mockReturnThis(),
        order: vi.fn().mockResolvedValue({ data: null, error: { message: "fail" } }),
      }),
    } as any;
    const result = await getTaskSessions(db, "repo-1");
    expect(result).toEqual([]);
  });

  it("uses custom limitDays for the date filter", async () => {
    let gteArg: string | undefined;
    const db = {
      from: vi.fn().mockReturnValue({
        select: vi.fn().mockReturnThis(),
        eq: vi.fn().mockReturnThis(),
        gte: vi.fn().mockImplementation((_col: string, val: string) => {
          gteArg = val;
          return { order: vi.fn().mockResolvedValue({ data: [], error: null }) };
        }),
      }),
    } as any;
    const before = Date.now();
    await getTaskSessions(db, "repo-1", 7);
    const expectedSince = new Date(before - 7 * 24 * 60 * 60 * 1000);
    // Allow 1 second tolerance for test execution time
    expect(new Date(gteArg!).getTime()).toBeGreaterThanOrEqual(expectedSince.getTime() - 1000);
    expect(new Date(gteArg!).getTime()).toBeLessThanOrEqual(expectedSince.getTime() + 1000);
  });
});
