# Phase 3: Session Continuity — Implementation Plan

> **For agentic workers:** REQUIRED: Use superpowers:subagent-driven-development (if subagents available) or superpowers:executing-plans to implement this plan. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add session tracking, snapshots, and project memory to sensei so agents can recover from interruptions and carry knowledge forward across sessions.

**Architecture:** Three new Supabase tables (`sessions`, `snapshots`, `memory_items`) back an engine session layer (`session-manager.ts`, `snapshot.ts`, `memory.ts`). The MCP server holds a `sessionId` in its in-memory closure, creates it on the first `get_session_context` call, and passes it to four new MCP tools (`take_snapshot`, `checkpoint`, `record_memory`, `close_memory`). A heartbeat wrapper updates `last_heartbeat` after each tool call. The dashboard gains a Sessions inspector route.

**Tech Stack:** Supabase JS client, vitest, Svelte 5 runes, Rokkit UI

> **Spec:** `docs/superpowers/specs/2026-03-13-phase3-session-continuity-design.md`

---

## File Structure

**New files:**
```
supabase/migrations/20260313000002_phase3_session_continuity.sql
packages/engine/src/session/session-manager.ts
packages/engine/src/session/session-manager.spec.ts
packages/engine/src/session/snapshot.ts
packages/engine/src/session/snapshot.spec.ts
packages/engine/src/session/memory.ts
packages/engine/src/session/memory.spec.ts
packages/server/src/tools/take-snapshot.ts
packages/server/src/tools/checkpoint.ts
packages/server/src/tools/record-memory.ts
packages/server/src/tools/close-memory.ts
apps/dashboard/src/routes/repos/[id]/sessions/+page.server.ts
apps/dashboard/src/routes/repos/[id]/sessions/+page.svelte
```

**Modified files:**
```
packages/engine/src/index.ts                               — export session/ modules
packages/server/src/tools/get-session-context.ts           — accept sessionId param, add interrupted + memory fields
packages/server/src/mcp-server.ts                          — session closure + heartbeat + 4 new tools
apps/dashboard/src/routes/repos/[id]/+page.svelte          — add "View Sessions →" link
```

---

## Chunk 1: Supabase Migration

### Task 1: Phase 3 Supabase Migration

**Files:**
- Create: `supabase/migrations/20260313000002_phase3_session_continuity.sql`
- Note: `database/ddl/table/sensei/sessions.ddl`, `snapshots.ddl`, `memory_items.ddl` were already created in a prior session — verify they exist before starting.

- [ ] **Step 1: Verify DDL source files exist**

```bash
ls database/ddl/table/sensei/sessions.ddl database/ddl/table/sensei/snapshots.ddl database/ddl/table/sensei/memory_items.ddl
```

Expected: all three files present.

- [ ] **Step 2: Create the migration file**

```sql
-- supabase/migrations/20260313000002_phase3_session_continuity.sql

-- Sessions: one per MCP server process
create table if not exists sensei.sessions (
  id             uuid        primary key default gen_random_uuid(),
  repo_id        uuid        not null references sensei.repos(id) on delete cascade,
  status         text        not null default 'active'
                             check (status in ('active', 'completed', 'crashed')),
  last_heartbeat timestamptz not null default now(),
  created_at     timestamptz not null default now()
);
create index if not exists sessions_repo_id_status_idx on sensei.sessions(repo_id, status);
create index if not exists sessions_last_heartbeat_idx on sensei.sessions(last_heartbeat)
  where status = 'active';

-- Snapshots: multiple per session
create table if not exists sensei.snapshots (
  id               uuid        primary key default gen_random_uuid(),
  session_id       uuid        not null references sensei.sessions(id) on delete cascade,
  repo_id          uuid        not null references sensei.repos(id) on delete cascade,
  kind             text        not null check (kind in ('manual', 'checkpoint')),
  progress_summary text        not null,
  next_step_hint   text,
  completed_steps  text[]      not null default '{}',
  in_flight_files  text[]      not null default '{}',
  worktree_refs    jsonb       not null default '[]',
  diff_stat_summary text,
  created_at       timestamptz not null default now()
);
create index if not exists snapshots_session_id_idx on sensei.snapshots(session_id, created_at desc);
create index if not exists snapshots_repo_id_idx    on sensei.snapshots(repo_id, created_at desc);

-- Memory items: project-scoped, survive session deletion
create table if not exists sensei.memory_items (
  id          uuid        primary key default gen_random_uuid(),
  repo_id     uuid        not null references sensei.repos(id) on delete cascade,
  session_id  uuid        references sensei.sessions(id) on delete set null,
  type        text        not null check (type in ('decision', 'pattern', 'question')),
  title       text        not null,
  content     text        not null,
  status      text        not null default 'open' check (status in ('open', 'closed')),
  resolution  text,
  closed_at   timestamptz,
  created_at  timestamptz not null default now()
);
create index if not exists memory_items_repo_id_idx    on sensei.memory_items(repo_id, type, status);
-- Additional index for session-scoped queries (present in DDL source files):
create index if not exists memory_items_session_id_idx on sensei.memory_items(session_id)
  where session_id is not null;

-- Grants (same pattern as Phase 1 and 2)
grant all on all tables in schema sensei to anon, authenticated, service_role;
grant all on all sequences in schema sensei to anon, authenticated, service_role;
grant execute on all functions in schema sensei to anon, authenticated, service_role;
```

- [ ] **Step 3: Apply migration**

```bash
cd /Users/Jerry/Developer/sensei
supabase db push
```

Expected: migration applies without errors.

- [ ] **Step 4: Commit**

```bash
git add supabase/migrations/20260313000002_phase3_session_continuity.sql
git commit -m "feat(db): phase 3 session continuity migration — sessions, snapshots, memory_items"
```

---

## Chunk 2: Engine Session Layer

> **Implementation order:** `snapshot.ts` (Task 3) must be implemented before `session-manager.ts` (Task 2) since session-manager imports `getLatestSnapshot` from snapshot. Follow the order: Task 3 → Task 2 → Task 4 → Task 5, then commit each as you go.

### Task 2: `session-manager.ts`

**Files:**
- Create: `packages/engine/src/session/session-manager.ts`
- Create: `packages/engine/src/session/session-manager.spec.ts`
- **Prerequisite:** Complete Task 3 (`snapshot.ts`) first.

- [ ] **Step 1: Write the failing tests**

```typescript
// packages/engine/src/session/session-manager.spec.ts
import { describe, it, expect, vi } from "vitest";
import { createSession, detectCrashedSessions, updateHeartbeat } from "./session-manager.js";

describe("createSession", () => {
  it("inserts a session row and returns id + createdAt", async () => {
    const db = {
      from: vi.fn().mockReturnValue({
        insert: vi.fn().mockReturnValue({
          select: vi.fn().mockReturnThis(),
          single: vi.fn().mockResolvedValue({
            data: { id: "sess-1", created_at: "2026-03-13T00:00:00Z" },
            error: null,
          }),
        }),
      }),
    } as any;
    const result = await createSession(db, "repo-1");
    expect(result.id).toBe("sess-1");
    expect(result.createdAt).toBe("2026-03-13T00:00:00Z");
    expect(db.from).toHaveBeenCalledWith("sessions");
  });

  it("throws on DB error", async () => {
    const db = {
      from: vi.fn().mockReturnValue({
        insert: vi.fn().mockReturnValue({
          select: vi.fn().mockReturnThis(),
          single: vi.fn().mockResolvedValue({ data: null, error: { message: "fail" } }),
        }),
      }),
    } as any;
    await expect(createSession(db, "repo-1")).rejects.toThrow("fail");
  });
});

describe("detectCrashedSessions", () => {
  it("returns [] when no idle sessions found", async () => {
    const db = {
      from: vi.fn().mockReturnValue({
        select: vi.fn().mockReturnThis(),
        eq: vi.fn().mockReturnThis(),
        lt: vi.fn().mockReturnThis(),
        order: vi.fn().mockReturnThis(),
        limit: vi.fn().mockResolvedValue({ data: [], error: null }),
      }),
    } as any;
    const result = await detectCrashedSessions(db, "repo-1");
    expect(result).toEqual([]);
  });

  it("returns [] on DB error (best-effort)", async () => {
    const db = {
      from: vi.fn().mockReturnValue({
        select: vi.fn().mockReturnThis(),
        eq: vi.fn().mockReturnThis(),
        lt: vi.fn().mockReturnThis(),
        order: vi.fn().mockReturnThis(),
        limit: vi.fn().mockResolvedValue({ data: null, error: { message: "db error" } }),
      }),
    } as any;
    const result = await detectCrashedSessions(db, "repo-1");
    expect(result).toEqual([]);
  });

  it("marks idle sessions as crashed and returns CrashedSession with lastHeartbeat", async () => {
    const idleSession = {
      id: "sess-old",
      created_at: "2026-03-13T00:00:00Z",
      last_heartbeat: "2026-03-13T00:00:00Z",
    };
    const snapshotRow = {
      id: "snap-1", kind: "manual",
      progress_summary: "mid-task", next_step_hint: null,
      completed_steps: [], in_flight_files: [], worktree_refs: [],
      diff_stat_summary: null, created_at: "2026-03-13T00:01:00Z",
    };
    // Mock: first from("sessions").select().eq().eq().lt().order().limit() returns the idle session
    // second from("sessions").update().in() marks it crashed
    // third from("snapshots").select().eq().order().limit().single() returns the snapshot
    let callCount = 0;
    const db = {
      from: vi.fn().mockImplementation((table: string) => {
        if (table === "sessions" && callCount === 0) {
          callCount++;
          return {
            select: vi.fn().mockReturnThis(),
            eq: vi.fn().mockReturnThis(),
            lt: vi.fn().mockReturnThis(),
            order: vi.fn().mockReturnThis(),
            limit: vi.fn().mockResolvedValue({ data: [idleSession], error: null }),
          };
        }
        if (table === "sessions" && callCount === 1) {
          callCount++;
          return {
            update: vi.fn().mockReturnValue({
              in: vi.fn().mockResolvedValue({ error: null }),
            }),
          };
        }
        // snapshots table
        return {
          select: vi.fn().mockReturnThis(),
          eq: vi.fn().mockReturnThis(),
          order: vi.fn().mockReturnThis(),
          limit: vi.fn().mockReturnValue({
            single: vi.fn().mockResolvedValue({ data: snapshotRow, error: null }),
          }),
        };
      }),
    } as any;

    const result = await detectCrashedSessions(db, "repo-1");
    expect(result).toHaveLength(1);
    expect(result[0].id).toBe("sess-old");
    expect(result[0].lastHeartbeat).toBe("2026-03-13T00:00:00Z");
    expect(result[0].latestSnapshot?.id).toBe("snap-1");
  });
});

describe("updateHeartbeat", () => {
  it("never throws, even on DB error", async () => {
    const db = {
      from: vi.fn().mockReturnValue({
        update: vi.fn().mockReturnValue({
          eq: vi.fn().mockResolvedValue({ error: { message: "fail" } }),
        }),
      }),
    } as any;
    await expect(updateHeartbeat(db, "sess-1")).resolves.toBeUndefined();
  });
});
```

- [ ] **Step 2: Run tests to verify they fail**

```bash
cd packages/engine && bunx vitest run src/session/session-manager.spec.ts
```

Expected: FAIL — module not found.

- [ ] **Step 3: Implement `session-manager.ts`**

```typescript
// packages/engine/src/session/session-manager.ts
import type { SupabaseClient } from "@supabase/supabase-js";
import { getLatestSnapshot } from "./snapshot.js";
import type { Snapshot } from "./snapshot.js";

export interface SessionInfo {
  id: string;
  createdAt: string;
}

export interface CrashedSession {
  id: string;
  createdAt: string;
  lastHeartbeat: string;  // last_heartbeat — surfaced as crashedAt in get_session_context
  latestSnapshot: Snapshot | null;
}

export async function createSession(db: SupabaseClient, repoId: string): Promise<SessionInfo> {
  const { data, error } = await db
    .from("sessions")
    .insert({ repo_id: repoId })
    .select("id,created_at")
    .single();
  if (error || !data) throw new Error(error?.message ?? "Failed to create session");
  return { id: data.id as string, createdAt: data.created_at as string };
}

export async function detectCrashedSessions(
  db: SupabaseClient,
  repoId: string,
  idleThresholdMs = 10 * 60 * 1000,
): Promise<CrashedSession[]> {
  try {
    const cutoff = new Date(Date.now() - idleThresholdMs).toISOString();
    const { data, error } = await db
      .from("sessions")
      .select("id,created_at,last_heartbeat")
      .eq("repo_id", repoId)
      .eq("status", "active")
      .lt("last_heartbeat", cutoff)
      .order("created_at", { ascending: false })
      .limit(10);

    if (error || !data || data.length === 0) return [];

    // Mark all as crashed
    await db
      .from("sessions")
      .update({ status: "crashed" })
      .in("id", (data as Array<{ id: string }>).map(s => s.id));

    // Fetch latest snapshot for each
    const results: CrashedSession[] = await Promise.all(
      (data as Array<{ id: string; created_at: string; last_heartbeat: string }>).map(async s => ({
        id: s.id,
        createdAt: s.created_at,
        lastHeartbeat: s.last_heartbeat,
        latestSnapshot: await getLatestSnapshot(db, s.id),
      }))
    );
    return results;
  } catch {
    return [];
  }
}

export async function updateHeartbeat(db: SupabaseClient, sessionId: string): Promise<void> {
  try {
    await db
      .from("sessions")
      .update({ last_heartbeat: new Date().toISOString() })
      .eq("id", sessionId);
  } catch {
    // Silent best-effort
  }
}
```

- [ ] **Step 4: Run tests to verify they pass**

```bash
cd packages/engine && bunx vitest run src/session/session-manager.spec.ts
```

Expected: PASS. (Note: `snapshot.ts` doesn't exist yet — create a stub or implement Task 3 first if needed. In practice, implement Task 3 first since `session-manager.ts` imports from `snapshot.js`.)

- [ ] **Step 5: Commit**

```bash
git add packages/engine/src/session/session-manager.ts packages/engine/src/session/session-manager.spec.ts
git commit -m "feat(engine): add session-manager — createSession, detectCrashedSessions, updateHeartbeat"
```

---

### Task 3: `snapshot.ts`

**Files:**
- Create: `packages/engine/src/session/snapshot.ts`
- Create: `packages/engine/src/session/snapshot.spec.ts`

> **Note:** Implement this BEFORE Task 2's Step 4, since `session-manager.ts` imports `getLatestSnapshot`.

- [ ] **Step 1: Write the failing tests**

```typescript
// packages/engine/src/session/snapshot.spec.ts
import { describe, it, expect, vi } from "vitest";
import { takeSnapshot, getLatestSnapshot } from "./snapshot.js";

const baseOpts = {
  kind: "manual" as const,
  progressSummary: "Working on auth module",
};

function makeDb(insertResult: any, selectResult: any) {
  const singleInsert = { select: vi.fn().mockReturnThis(), single: vi.fn().mockResolvedValue(insertResult) };
  const singleSelect = { single: vi.fn().mockResolvedValue(selectResult) };
  return {
    from: vi.fn().mockReturnValue({
      insert: vi.fn().mockReturnValue(singleInsert),
      select: vi.fn().mockReturnThis(),
      eq: vi.fn().mockReturnThis(),
      order: vi.fn().mockReturnThis(),
      limit: vi.fn().mockReturnValue(singleSelect),
    }),
  } as any;
}

const snapshotRow = {
  id: "snap-1",
  kind: "manual",
  progress_summary: "Working on auth module",
  next_step_hint: null,
  completed_steps: [],
  in_flight_files: [],
  worktree_refs: [],
  diff_stat_summary: null,
  created_at: "2026-03-13T00:00:00Z",
};

describe("takeSnapshot", () => {
  it("inserts snapshot and returns shaped Snapshot", async () => {
    const db = makeDb({ data: snapshotRow, error: null }, null);
    const result = await takeSnapshot(db, "sess-1", "repo-1", baseOpts);
    expect(result.id).toBe("snap-1");
    expect(result.kind).toBe("manual");
    expect(result.progressSummary).toBe("Working on auth module");
    expect(result.nextStepHint).toBeNull();
    expect(result.completedSteps).toEqual([]);
    expect(result.createdAt).toBe("2026-03-13T00:00:00Z");
  });

  it("throws on DB error", async () => {
    const db = makeDb({ data: null, error: { message: "fail" } }, null);
    await expect(takeSnapshot(db, "sess-1", "repo-1", baseOpts)).rejects.toThrow("fail");
  });
});

describe("getLatestSnapshot", () => {
  it("returns null when no snapshots exist", async () => {
    const db = {
      from: vi.fn().mockReturnValue({
        select: vi.fn().mockReturnThis(),
        eq: vi.fn().mockReturnThis(),
        order: vi.fn().mockReturnThis(),
        limit: vi.fn().mockReturnValue({ single: vi.fn().mockResolvedValue({ data: null, error: null }) }),
      }),
    } as any;
    const result = await getLatestSnapshot(db, "sess-1");
    expect(result).toBeNull();
  });

  it("returns shaped Snapshot when found", async () => {
    const db = {
      from: vi.fn().mockReturnValue({
        select: vi.fn().mockReturnThis(),
        eq: vi.fn().mockReturnThis(),
        order: vi.fn().mockReturnThis(),
        limit: vi.fn().mockReturnValue({ single: vi.fn().mockResolvedValue({ data: snapshotRow, error: null }) }),
      }),
    } as any;
    const result = await getLatestSnapshot(db, "sess-1");
    expect(result?.id).toBe("snap-1");
    expect(result?.progressSummary).toBe("Working on auth module");
  });
});
```

- [ ] **Step 2: Run tests to verify they fail**

```bash
cd packages/engine && bunx vitest run src/session/snapshot.spec.ts
```

Expected: FAIL — module not found.

- [ ] **Step 3: Implement `snapshot.ts`**

```typescript
// packages/engine/src/session/snapshot.ts
import type { SupabaseClient } from "@supabase/supabase-js";

export interface SnapshotOptions {
  kind: "manual" | "checkpoint";
  progressSummary: string;
  nextStepHint?: string;
  completedSteps?: string[];
  inFlightFiles?: string[];
  worktreeRefs?: Array<{ branch: string; path: string; status: string }>;
  diffStatSummary?: string;
}

export interface Snapshot {
  id: string;
  kind: "manual" | "checkpoint";
  progressSummary: string;
  nextStepHint: string | null;
  completedSteps: string[];
  inFlightFiles: string[];
  worktreeRefs: Array<{ branch: string; path: string; status: string }>;
  diffStatSummary: string | null;
  createdAt: string;
}

function shapeSnapshot(row: Record<string, unknown>): Snapshot {
  return {
    id: row.id as string,
    kind: row.kind as "manual" | "checkpoint",
    progressSummary: row.progress_summary as string,
    nextStepHint: (row.next_step_hint as string | null) ?? null,
    completedSteps: (row.completed_steps as string[]) ?? [],
    inFlightFiles: (row.in_flight_files as string[]) ?? [],
    worktreeRefs: (row.worktree_refs as Array<{ branch: string; path: string; status: string }>) ?? [],
    diffStatSummary: (row.diff_stat_summary as string | null) ?? null,
    createdAt: row.created_at as string,
  };
}

export async function takeSnapshot(
  db: SupabaseClient,
  sessionId: string,
  repoId: string,
  opts: SnapshotOptions,
): Promise<Snapshot> {
  const { data, error } = await db
    .from("snapshots")
    .insert({
      session_id: sessionId,
      repo_id: repoId,
      kind: opts.kind,
      progress_summary: opts.progressSummary,
      next_step_hint: opts.nextStepHint ?? null,
      completed_steps: opts.completedSteps ?? [],
      in_flight_files: opts.inFlightFiles ?? [],
      worktree_refs: opts.worktreeRefs ?? [],
      diff_stat_summary: opts.diffStatSummary ?? null,
    })
    .select()
    .single();
  if (error || !data) throw new Error(error?.message ?? "Failed to save snapshot");
  return shapeSnapshot(data as Record<string, unknown>);
}

export async function getLatestSnapshot(db: SupabaseClient, sessionId: string): Promise<Snapshot | null> {
  const { data } = await db
    .from("snapshots")
    .select("*")
    .eq("session_id", sessionId)
    .order("created_at", { ascending: false })
    .limit(1)
    .single();
  if (!data) return null;
  return shapeSnapshot(data as Record<string, unknown>);
}
```

- [ ] **Step 4: Run tests to verify they pass**

```bash
cd packages/engine && bunx vitest run src/session/snapshot.spec.ts
```

Expected: PASS.

- [ ] **Step 5: Now run session-manager tests too**

```bash
cd packages/engine && bunx vitest run src/session/session-manager.spec.ts
```

Expected: PASS.

- [ ] **Step 6: Commit**

```bash
git add packages/engine/src/session/snapshot.ts packages/engine/src/session/snapshot.spec.ts packages/engine/src/session/session-manager.ts packages/engine/src/session/session-manager.spec.ts
git commit -m "feat(engine): add snapshot.ts and session-manager.ts with tests"
```

---

### Task 4: `memory.ts`

**Files:**
- Create: `packages/engine/src/session/memory.ts`
- Create: `packages/engine/src/session/memory.spec.ts`

- [ ] **Step 1: Write the failing tests**

```typescript
// packages/engine/src/session/memory.spec.ts
import { describe, it, expect, vi } from "vitest";
import { recordMemory, closeMemory, getMemoryItems } from "./memory.js";

const memRow = {
  id: "mem-1",
  type: "decision",
  title: "Use optimistic locking",
  content: "Apply optimistic locking for invoice updates to avoid contention",
  status: "open",
  resolution: null,
  closed_at: null,
  created_at: "2026-03-13T00:00:00Z",
};

function makeSingleDb(result: any) {
  return {
    from: vi.fn().mockReturnValue({
      insert: vi.fn().mockReturnValue({
        select: vi.fn().mockReturnThis(),
        single: vi.fn().mockResolvedValue(result),
      }),
      update: vi.fn().mockReturnValue({
        eq: vi.fn().mockReturnThis(),
        select: vi.fn().mockReturnThis(),
        single: vi.fn().mockResolvedValue(result),
      }),
      select: vi.fn().mockReturnThis(),
      eq: vi.fn().mockReturnThis(),
      order: vi.fn().mockReturnThis(),
      limit: vi.fn().mockResolvedValue({ data: [memRow], error: null }),
    }),
  } as any;
}

describe("recordMemory", () => {
  it("inserts memory item and returns shaped MemoryItem", async () => {
    const db = makeSingleDb({ data: memRow, error: null });
    const result = await recordMemory(db, "repo-1", "sess-1", {
      type: "decision",
      title: "Use optimistic locking",
      content: "Apply optimistic locking for invoice updates to avoid contention",
    });
    expect(result.id).toBe("mem-1");
    expect(result.type).toBe("decision");
    expect(result.status).toBe("open");
    expect(result.resolution).toBeNull();
    expect(result.closedAt).toBeNull();
  });

  it("throws on DB error", async () => {
    const db = makeSingleDb({ data: null, error: { message: "fail" } });
    await expect(
      recordMemory(db, "repo-1", "sess-1", { type: "decision", title: "t", content: "c" })
    ).rejects.toThrow("fail");
  });
});

describe("closeMemory", () => {
  it("updates item and returns closed MemoryItem", async () => {
    const closedRow = { ...memRow, status: "closed", resolution: "resolved via X", closed_at: "2026-03-13T01:00:00Z" };
    // First call: select to check current status (returns open item)
    // Second call: update to close it (returns closed item)
    let callCount = 0;
    const db = {
      from: vi.fn().mockImplementation(() => {
        callCount++;
        if (callCount === 1) {
          // check status
          return {
            select: vi.fn().mockReturnThis(),
            eq: vi.fn().mockReturnThis(),
            single: vi.fn().mockResolvedValue({ data: memRow, error: null }),
          };
        }
        // update
        return {
          update: vi.fn().mockReturnValue({
            eq: vi.fn().mockReturnThis(),
            select: vi.fn().mockReturnThis(),
            single: vi.fn().mockResolvedValue({ data: closedRow, error: null }),
          }),
        };
      }),
    } as any;
    const result = await closeMemory(db, "mem-1", "resolved via X");
    expect(result.status).toBe("closed");
    expect(result.resolution).toBe("resolved via X");
    expect(result.closedAt).toBe("2026-03-13T01:00:00Z");
  });

  it("throws if item not found (update returns no data)", async () => {
    let callCount = 0;
    const db = {
      from: vi.fn().mockImplementation(() => {
        callCount++;
        if (callCount === 1) {
          // select: item not found
          return {
            select: vi.fn().mockReturnThis(),
            eq: vi.fn().mockReturnThis(),
            single: vi.fn().mockResolvedValue({ data: null, error: null }),
          };
        }
        // update: also not found
        return {
          update: vi.fn().mockReturnValue({
            eq: vi.fn().mockReturnThis(),
            select: vi.fn().mockReturnThis(),
            single: vi.fn().mockResolvedValue({ data: null, error: { message: "no rows" } }),
          }),
        };
      }),
    } as any;
    await expect(closeMemory(db, "mem-1", "resolution")).rejects.toThrow();
  });

  it("throws if item already closed", async () => {
    const closedRow = { ...memRow, status: "closed" };
    const db = {
      from: vi.fn().mockReturnValue({
        select: vi.fn().mockReturnThis(),
        eq: vi.fn().mockReturnThis(),
        single: vi.fn().mockResolvedValue({ data: closedRow, error: null }),
      }),
    } as any;
    await expect(closeMemory(db, "mem-1", "resolution")).rejects.toThrow("already closed");
  });
});

describe("getMemoryItems", () => {
  it("returns shaped MemoryItems", async () => {
    const db = {
      from: vi.fn().mockReturnValue({
        select: vi.fn().mockReturnThis(),
        eq: vi.fn().mockReturnThis(),
        order: vi.fn().mockResolvedValue({ data: [memRow], error: null }),
      }),
    } as any;
    const result = await getMemoryItems(db, "repo-1");
    expect(result).toHaveLength(1);
    expect(result[0].id).toBe("mem-1");
  });

  it("returns [] on error (never throws)", async () => {
    const db = {
      from: vi.fn().mockReturnValue({
        select: vi.fn().mockReturnThis(),
        eq: vi.fn().mockReturnThis(),
        order: vi.fn().mockResolvedValue({ data: null, error: { message: "fail" } }),
      }),
    } as any;
    const result = await getMemoryItems(db, "repo-1");
    expect(result).toEqual([]);
  });
});
```

- [ ] **Step 2: Run tests to verify they fail**

```bash
cd packages/engine && bunx vitest run src/session/memory.spec.ts
```

Expected: FAIL — module not found.

- [ ] **Step 3: Implement `memory.ts`**

```typescript
// packages/engine/src/session/memory.ts
import type { SupabaseClient } from "@supabase/supabase-js";

export interface MemoryItem {
  id: string;
  type: "decision" | "pattern" | "question";
  title: string;
  content: string;
  status: "open" | "closed";
  resolution: string | null;
  closedAt: string | null;  // set when status=closed
  createdAt: string;
}

function shapeMemoryItem(row: Record<string, unknown>): MemoryItem {
  return {
    id: row.id as string,
    type: row.type as "decision" | "pattern" | "question",
    title: row.title as string,
    content: row.content as string,
    status: row.status as "open" | "closed",
    resolution: (row.resolution as string | null) ?? null,
    closedAt: (row.closed_at as string | null) ?? null,
    createdAt: row.created_at as string,
  };
}

export async function recordMemory(
  db: SupabaseClient,
  repoId: string,
  sessionId: string,
  opts: { type: "decision" | "pattern" | "question"; title: string; content: string },
): Promise<MemoryItem> {
  const { data, error } = await db
    .from("memory_items")
    .insert({ repo_id: repoId, session_id: sessionId, type: opts.type, title: opts.title, content: opts.content })
    .select()
    .single();
  if (error || !data) throw new Error(error?.message ?? "Failed to record memory");
  return shapeMemoryItem(data as Record<string, unknown>);
}

export async function closeMemory(
  db: SupabaseClient,
  itemId: string,
  resolution: string,
): Promise<MemoryItem> {
  // Check current status first
  const { data: existing } = await db
    .from("memory_items")
    .select("status")
    .eq("id", itemId)
    .single();
  if (existing && (existing as Record<string, unknown>).status === "closed") {
    throw new Error("Memory item already closed");
  }

  const { data, error } = await db
    .from("memory_items")
    .update({ status: "closed", resolution, closed_at: new Date().toISOString() })
    .eq("id", itemId)
    .select()
    .single();
  if (error || !data) throw new Error(error?.message ?? "Memory item not found");
  return shapeMemoryItem(data as Record<string, unknown>);
}

export async function getMemoryItems(db: SupabaseClient, repoId: string): Promise<MemoryItem[]> {
  try {
    const { data, error } = await db
      .from("memory_items")
      .select("*")
      .eq("repo_id", repoId)
      .order("created_at", { ascending: false });
    if (error || !data) return [];
    return (data as Array<Record<string, unknown>>).map(shapeMemoryItem);
  } catch {
    return [];
  }
}
```

- [ ] **Step 4: Run tests to verify they pass**

```bash
cd packages/engine && bunx vitest run src/session/memory.spec.ts
```

Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add packages/engine/src/session/memory.ts packages/engine/src/session/memory.spec.ts
git commit -m "feat(engine): add memory.ts — recordMemory, closeMemory, getMemoryItems"
```

---

### Task 5: Export Session Modules from `packages/engine/src/index.ts`

**Files:**
- Modify: `packages/engine/src/index.ts`

- [ ] **Step 1: Add session exports**

Append to `packages/engine/src/index.ts`:

```typescript
export * from "./session/session-manager.js";
export * from "./session/snapshot.js";
export * from "./session/memory.js";
```

- [ ] **Step 2: Run all engine tests**

```bash
cd packages/engine && bunx vitest run
```

Expected: all tests pass (including the 3 new session test files).

- [ ] **Step 3: Commit**

```bash
git add packages/engine/src/index.ts
git commit -m "feat(engine): export session modules from index"
```

---

## Chunk 3: MCP Tools

### Task 6: Enhance `get-session-context.ts`

**Files:**
- Modify: `packages/server/src/tools/get-session-context.ts`

The current `getSessionContext` takes `(client, repoId, repoPath)`. The enhanced version accepts a `sessionId` (already created by the caller in the mcp-server closure) and also calls `detectCrashedSessions` and `getMemoryItems`.

- [ ] **Step 1: Replace the contents of `get-session-context.ts`**

```typescript
// packages/server/src/tools/get-session-context.ts
import type { SupabaseClient } from "@supabase/supabase-js";
import { detectCrashedSessions, type CrashedSession } from "@sensei/engine";
import { getMemoryItems, type MemoryItem } from "@sensei/engine";
import type { Snapshot } from "@sensei/engine";

export interface SessionContextResult {
  repo_name: string;
  repo_path: string;
  symbol_count: number;
  file_count: number;
  last_indexed_at: string | null;
  stack: string[];
  session_id: string;
  interrupted: Array<{
    sessionId: string;
    crashedAt: string;
    snapshot: Snapshot | null;
  }>;
  memory: {
    decisions: MemoryItem[];
    patterns: MemoryItem[];
    openQuestions: MemoryItem[];
  };
  message: string;
}

export async function getSessionContext(
  client: SupabaseClient,
  repoId: string,
  repoPath: string,
  sessionId: string,
): Promise<SessionContextResult> {
  const { data: repo, error } = await client.from("repos").select("*").eq("id", repoId).single();
  if (error || !repo) throw new Error(`Repo not found: ${error?.message ?? "no data"}`);

  const { count: symbolCount } = await client
    .from("symbols")
    .select("*", { count: "exact", head: true })
    .eq("repo_id", repoId);

  const { count: fileCount } = await client
    .from("scan_state")
    .select("*", { count: "exact", head: true })
    .eq("repo_id", repoId);

  const crashed: CrashedSession[] = await detectCrashedSessions(client, repoId);

  const allMemory: MemoryItem[] = await getMemoryItems(client, repoId);
  const decisions = allMemory.filter(m => m.type === "decision");
  const patterns = allMemory.filter(m => m.type === "pattern");
  const openQuestions = allMemory.filter(m => m.type === "question" && m.status === "open");

  const interruptedMsg = crashed.length > 0
    ? ` ${crashed.length} interrupted session(s) detected — check interrupted[] for recovery context.`
    : "";

  return {
    repo_name: repo?.name ?? "unknown",
    repo_path: repoPath,
    symbol_count: symbolCount ?? 0,
    file_count: fileCount ?? 0,
    last_indexed_at: repo?.last_indexed_at ?? null,
    stack: repo?.stack ?? [],
    session_id: sessionId,
    interrupted: crashed.map(c => ({
      sessionId: c.id,
      crashedAt: c.lastHeartbeat,  // last_heartbeat of the crashed session, per spec
      snapshot: c.latestSnapshot,
    })),
    memory: { decisions, patterns, openQuestions },
    message: `Repo "${repo?.name ?? "unknown"}" — ${symbolCount ?? 0} symbols across ${fileCount ?? 0} files.${interruptedMsg} Call search() to find code.`,
  };
}
```

- [ ] **Step 2: Verify no TypeScript errors**

```bash
cd packages/server && bunx tsc --noEmit
```

Expected: no errors.

- [ ] **Step 3: Commit**

```bash
git add packages/server/src/tools/get-session-context.ts
git commit -m "feat(server): enhance get-session-context with session_id, interrupted, memory fields"
```

---

### Task 7: `take-snapshot.ts`

**Files:**
- Create: `packages/server/src/tools/take-snapshot.ts`

- [ ] **Step 1: Create the tool handler**

```typescript
// packages/server/src/tools/take-snapshot.ts
import type { SupabaseClient } from "@supabase/supabase-js";
import { takeSnapshot as engineTakeSnapshot } from "@sensei/engine";

interface TakeSnapshotParams {
  progress_summary: string;
  next_step_hint?: string;
  in_flight_files?: string[];
  completed_steps?: string[];
  worktree_refs?: Array<{ branch: string; path: string; status: string }>;
  diff_stat_summary?: string;
}

export async function takeSnapshotTool(
  client: SupabaseClient,
  sessionId: string,
  repoId: string,
  params: TakeSnapshotParams,
) {
  const snapshot = await engineTakeSnapshot(client, sessionId, repoId, {
    kind: "manual",
    progressSummary: params.progress_summary,
    nextStepHint: params.next_step_hint,
    completedSteps: params.completed_steps,
    inFlightFiles: params.in_flight_files,
    worktreeRefs: params.worktree_refs,
    diffStatSummary: params.diff_stat_summary,
  });
  return {
    id: snapshot.id,
    kind: snapshot.kind,
    progressSummary: snapshot.progressSummary,
    createdAt: snapshot.createdAt,
  };
}
```

- [ ] **Step 2: Verify no TypeScript errors**

```bash
cd packages/server && bunx tsc --noEmit
```

Expected: no errors.

- [ ] **Step 3: Commit**

```bash
git add packages/server/src/tools/take-snapshot.ts
git commit -m "feat(server): add take-snapshot tool handler"
```

---

### Task 8: `checkpoint.ts`

**Files:**
- Create: `packages/server/src/tools/checkpoint.ts`

- [ ] **Step 1: Create the tool handler**

```typescript
// packages/server/src/tools/checkpoint.ts
import type { SupabaseClient } from "@supabase/supabase-js";
import { takeSnapshot } from "@sensei/engine";

interface CheckpointParams {
  task_summary: string;
  completed_steps?: string[];
}

export async function checkpointTool(
  client: SupabaseClient,
  sessionId: string,
  repoId: string,
  params: CheckpointParams,
) {
  // Write checkpoint snapshot
  const snapshot = await takeSnapshot(client, sessionId, repoId, {
    kind: "checkpoint",
    progressSummary: params.task_summary,
    completedSteps: params.completed_steps,
  });

  // Mark session completed
  await client
    .from("sessions")
    .update({ status: "completed" })
    .eq("id", sessionId);

  return {
    id: snapshot.id,
    kind: snapshot.kind,
    progressSummary: snapshot.progressSummary,
    createdAt: snapshot.createdAt,
  };
}
```

- [ ] **Step 2: Verify no TypeScript errors**

```bash
cd packages/server && bunx tsc --noEmit
```

Expected: no errors.

- [ ] **Step 3: Commit**

```bash
git add packages/server/src/tools/checkpoint.ts
git commit -m "feat(server): add checkpoint tool handler"
```

---

### Task 9: `record-memory.ts`

**Files:**
- Create: `packages/server/src/tools/record-memory.ts`

- [ ] **Step 1: Create the tool handler**

```typescript
// packages/server/src/tools/record-memory.ts
import type { SupabaseClient } from "@supabase/supabase-js";
import { recordMemory as engineRecordMemory } from "@sensei/engine";

interface RecordMemoryParams {
  type: "decision" | "pattern" | "question";
  title: string;
  content: string;
}

export async function recordMemoryTool(
  client: SupabaseClient,
  repoId: string,
  sessionId: string,
  params: RecordMemoryParams,
) {
  const item = await engineRecordMemory(client, repoId, sessionId, params);
  return {
    id: item.id,
    type: item.type,
    title: item.title,
    status: item.status,
    createdAt: item.createdAt,
  };
}
```

- [ ] **Step 2: Verify no TypeScript errors**

```bash
cd packages/server && bunx tsc --noEmit
```

Expected: no errors.

- [ ] **Step 3: Commit**

```bash
git add packages/server/src/tools/record-memory.ts
git commit -m "feat(server): add record-memory tool handler"
```

---

### Task 10: `close-memory.ts`

**Files:**
- Create: `packages/server/src/tools/close-memory.ts`

- [ ] **Step 1: Create the tool handler**

```typescript
// packages/server/src/tools/close-memory.ts
import type { SupabaseClient } from "@supabase/supabase-js";
import { closeMemory as engineCloseMemory } from "@sensei/engine";

interface CloseMemoryParams {
  id: string;
  resolution: string;
}

export async function closeMemoryTool(
  client: SupabaseClient,
  params: CloseMemoryParams,
) {
  const item = await engineCloseMemory(client, params.id, params.resolution);
  return {
    id: item.id,
    type: item.type,
    title: item.title,
    status: item.status,
    resolution: item.resolution,
    closedAt: item.closedAt,
  };
}
```

- [ ] **Step 2: Verify no TypeScript errors**

```bash
cd packages/server && bunx tsc --noEmit
```

Expected: no errors.

- [ ] **Step 3: Commit**

```bash
git add packages/server/src/tools/close-memory.ts
git commit -m "feat(server): add close-memory tool handler"
```

---

### Task 11: Update `mcp-server.ts` — Session Closure, Heartbeat, 4 New Tools

**Files:**
- Modify: `packages/server/src/mcp-server.ts`

This is the most complex task. The `mcp-server.ts` gains:
1. A `sessionId` closure variable (null until first `get_session_context` call)
2. A `beat()` helper that calls `updateHeartbeat` silently
3. Updated `get_session_context` handler: creates session on first call, passes sessionId to `getSessionContext`
4. Four new tool registrations: `take_snapshot`, `checkpoint`, `record_memory`, `close_memory`

- [ ] **Step 1: Add imports and session closure to `mcp-server.ts`**

Add to the top imports section:

```typescript
import { createSession, updateHeartbeat } from "@sensei/engine";
import { takeSnapshotTool } from "./tools/take-snapshot.js";
import { checkpointTool } from "./tools/checkpoint.js";
import { recordMemoryTool } from "./tools/record-memory.js";
import { closeMemoryTool } from "./tools/close-memory.js";
```

Inside `createSenseiMcpServer`, after `const getBackend = ...`:

```typescript
  // Session state — stored per MCP server process
  let sessionId: string | null = null;

  const beat = (client: any) => {
    if (sessionId) updateHeartbeat(client, sessionId).catch(() => {});
  };
```

- [ ] **Step 2: Update the `get_session_context` tool handler**

Replace the existing handler body:

```typescript
  server.tool(
    "get_session_context",
    "Get orientation context for the current repo — symbol count, stack, last indexed timestamp, interrupted sessions, and project memory",
    {},
    async () => {
      try {
        const client = await getClient();
        if (!client) return { content: [{ type: "text", text: "Error: Supabase client not configured. Run sensei init first." }] };

        // Create session on first call; reuse on subsequent calls (idempotent)
        if (!sessionId) {
          const session = await createSession(client as any, opts.repoId);
          sessionId = session.id;
        }

        const result = await getSessionContext(client as any, opts.repoId, opts.repoPath, sessionId);
        beat(client);
        return { content: [{ type: "text", text: JSON.stringify(result, null, 2) }] };
      } catch (err) {
        return { content: [{ type: "text", text: `Error: ${err instanceof Error ? err.message : String(err)}` }], isError: true };
      }
    }
  );
```

- [ ] **Step 3: Add heartbeat to existing tools**

After each existing tool call (search, load_context, context_pack, recommend_next, token_stats), add `beat(client);` before the `return` statement. Pattern:

```typescript
        const result = await search(client as any, opts.repoId, query, limit);
        beat(client);  // ← add this line
        return { content: [{ type: "text", text: JSON.stringify(result, null, 2) }] };
```

Apply the same pattern to all 5 existing tools.

- [ ] **Step 4: Register the 4 new tools**

Add after the `token_stats` tool registration:

```typescript
  server.tool(
    "take_snapshot",
    "Save a snapshot of current progress — use at step boundaries so the next session can recover if interrupted",
    {
      progress_summary: z.string().describe("What you are doing right now"),
      next_step_hint: z.string().optional().describe("What to do next if interrupted"),
      in_flight_files: z.array(z.string()).optional().describe("Files currently being modified"),
      completed_steps: z.array(z.string()).optional().describe("Steps finished so far this task"),
      worktree_refs: z.array(z.object({ branch: z.string(), path: z.string(), status: z.string() })).optional().describe("Active git worktrees"),
      diff_stat_summary: z.string().optional().describe("e.g. '8 files changed, +142 -31'"),
    },
    async (params) => {
      try {
        const client = await getClient();
        if (!client) return { content: [{ type: "text", text: "Error: Supabase client not configured." }] };
        if (!sessionId) return { content: [{ type: "text", text: "Error: No active session. Call get_session_context first." }], isError: true };
        const result = await takeSnapshotTool(client as any, sessionId, opts.repoId, params);
        beat(client);
        return { content: [{ type: "text", text: JSON.stringify(result, null, 2) }] };
      } catch (err) {
        return { content: [{ type: "text", text: `Error: ${err instanceof Error ? err.message : String(err)}` }], isError: true };
      }
    }
  );

  server.tool(
    "checkpoint",
    "Mark a task complete — writes a final snapshot and closes the session. Call when a coherent unit of work is done.",
    {
      task_summary: z.string().describe("What was accomplished"),
      completed_steps: z.array(z.string()).optional().describe("Final list of completed steps"),
    },
    async (params) => {
      try {
        const client = await getClient();
        if (!client) return { content: [{ type: "text", text: "Error: Supabase client not configured." }] };
        if (!sessionId) return { content: [{ type: "text", text: "Error: No active session. Call get_session_context first." }], isError: true };
        const result = await checkpointTool(client as any, sessionId, opts.repoId, params);
        beat(client);
        return { content: [{ type: "text", text: JSON.stringify(result, null, 2) }] };
      } catch (err) {
        return { content: [{ type: "text", text: `Error: ${err instanceof Error ? err.message : String(err)}` }], isError: true };
      }
    }
  );

  server.tool(
    "record_memory",
    "Store a decision, pattern, or open question in project memory — survives across sessions",
    {
      type: z.enum(["decision", "pattern", "question"]).describe("decision=architectural choice, pattern=coding convention, question=open question to resolve"),
      title: z.string().describe("Short label, e.g. 'Use optimistic locking for invoice updates'"),
      content: z.string().describe("Full description"),
    },
    async (params) => {
      try {
        const client = await getClient();
        if (!client) return { content: [{ type: "text", text: "Error: Supabase client not configured." }] };
        if (!sessionId) return { content: [{ type: "text", text: "Error: No active session. Call get_session_context first." }], isError: true };
        const result = await recordMemoryTool(client as any, opts.repoId, sessionId, params);
        beat(client);
        return { content: [{ type: "text", text: JSON.stringify(result, null, 2) }] };
      } catch (err) {
        return { content: [{ type: "text", text: `Error: ${err instanceof Error ? err.message : String(err)}` }], isError: true };
      }
    }
  );

  server.tool(
    "close_memory",
    "Resolve an open question in project memory",
    {
      id: z.string().describe("Memory item ID returned by record_memory"),
      resolution: z.string().describe("How the question was resolved"),
    },
    async (params) => {
      try {
        const client = await getClient();
        if (!client) return { content: [{ type: "text", text: "Error: Supabase client not configured." }] };
        const result = await closeMemoryTool(client as any, params);
        beat(client);
        return { content: [{ type: "text", text: JSON.stringify(result, null, 2) }] };
      } catch (err) {
        return { content: [{ type: "text", text: `Error: ${err instanceof Error ? err.message : String(err)}` }], isError: true };
      }
    }
  );
```

- [ ] **Step 5: Verify no TypeScript errors**

```bash
cd packages/server && bunx tsc --noEmit
```

Expected: no errors.

- [ ] **Step 6: Run all server tests**

```bash
cd packages/server && bunx vitest run
```

Expected: all existing tests pass.

- [ ] **Step 7: Commit**

```bash
git add packages/server/src/mcp-server.ts packages/server/src/tools/take-snapshot.ts packages/server/src/tools/checkpoint.ts packages/server/src/tools/record-memory.ts packages/server/src/tools/close-memory.ts
git commit -m "feat(server): add session closure, heartbeat, and 4 new MCP tools (take_snapshot, checkpoint, record_memory, close_memory)"
```

---

## Chunk 4: Dashboard

### Task 12: Sessions Page — Server Load

**Files:**
- Create: `apps/dashboard/src/routes/repos/[id]/sessions/+page.server.ts`

The load function queries sessions with snapshot count and memory item count per session.

- [ ] **Step 1: Create the server load file**

```typescript
// apps/dashboard/src/routes/repos/[id]/sessions/+page.server.ts
import type { PageServerLoad } from './$types';
import { getDb } from '$lib/server/db';
import { error } from '@sveltejs/kit';

export const load: PageServerLoad = async ({ params }) => {
  const db = getDb();

  const { data: repo } = await db
    .from('repos')
    .select('id,name')
    .eq('id', params.id)
    .single();

  if (!repo) throw error(404, 'Repo not found');

  const { data: sessions } = await db
    .from('sessions')
    .select('id,status,last_heartbeat,created_at')
    .eq('repo_id', params.id)
    .order('created_at', { ascending: false })
    .limit(50);

  // For each session, fetch its snapshots and memory items
  const sessionIds = ((sessions ?? []) as Array<{ id: string }>).map(s => s.id);

  const { data: snapshots } = sessionIds.length > 0
    ? await db
        .from('snapshots')
        .select('id,session_id,kind,progress_summary,created_at')
        .in('session_id', sessionIds)
        .order('created_at', { ascending: false })
    : { data: [] };

  const { data: memoryItems } = sessionIds.length > 0
    ? await db
        .from('memory_items')
        .select('id,session_id,type,title,status')
        .in('session_id', sessionIds)
        .order('created_at', { ascending: false })
    : { data: [] };

  const snapshotsBySession = new Map<string, typeof snapshots>();
  for (const snap of (snapshots ?? []) as Array<{ session_id: string } & Record<string, unknown>>) {
    const arr = snapshotsBySession.get(snap.session_id) ?? [];
    arr.push(snap as any);
    snapshotsBySession.set(snap.session_id, arr);
  }

  const memoryBySession = new Map<string, typeof memoryItems>();
  for (const mem of (memoryItems ?? []) as Array<{ session_id: string | null } & Record<string, unknown>>) {
    if (!mem.session_id) continue;
    const arr = memoryBySession.get(mem.session_id) ?? [];
    arr.push(mem as any);
    memoryBySession.set(mem.session_id, arr);
  }

  return {
    repo: repo as { id: string; name: string },
    sessions: ((sessions ?? []) as Array<Record<string, unknown>>).map(s => ({
      id: s.id as string,
      status: s.status as string,
      lastHeartbeat: s.last_heartbeat as string,
      createdAt: s.created_at as string,
      snapshots: (snapshotsBySession.get(s.id as string) ?? []).map((sn: any) => ({
        id: sn.id as string,
        kind: sn.kind as string,
        progressSummary: sn.progress_summary as string,
        createdAt: sn.created_at as string,
      })),
      memoryItems: (memoryBySession.get(s.id as string) ?? []).map((m: any) => ({
        id: m.id as string,
        type: m.type as string,
        title: m.title as string,
        status: m.status as string,
      })),
    })),
  };
};
```

- [ ] **Step 2: Verify no TypeScript errors in dashboard**

```bash
cd apps/dashboard && bunx svelte-check --tsconfig ./tsconfig.json 2>&1 | head -30
```

Expected: no errors related to the new file.

- [ ] **Step 3: Commit**

```bash
git add apps/dashboard/src/routes/repos/\[id\]/sessions/+page.server.ts
git commit -m "feat(dashboard): add sessions page server load"
```

---

### Task 13: Sessions Page — UI

**Files:**
- Create: `apps/dashboard/src/routes/repos/[id]/sessions/+page.svelte`

Pattern: same expandable list as context-packs page. Status badge uses text class names (active=green, completed=grey, crashed=red). Expanding a session shows snapshot timeline + memory items, each in a Rokkit Table.

- [ ] **Step 1: Create the Svelte page**

```svelte
<script lang="ts">
  import { Table } from '@rokkit/ui';
  import type { PageData } from './$types';

  const { data } = $props();

  let expandedId = $state<string | null>(null);

  const toggle = (id: string) => { expandedId = expandedId === id ? null : id; };
  const fmt = (iso: string) => new Date(iso).toLocaleString();

  const snapshotColumns = [
    { name: 'kind',            label: 'Kind',     sortable: true },
    { name: 'progressSummary', label: 'Progress', sortable: false },
    { name: 'createdAt',       label: 'Created',  sortable: true },
  ];

  const memoryColumns = [
    { name: 'type',   label: 'Type',   sortable: true },
    { name: 'title',  label: 'Title',  sortable: false },
    { name: 'status', label: 'Status', sortable: true },
  ];

  function snapshotRows(snaps: any[]) {
    return snaps.map(s => ({ ...s, createdAt: fmt(s.createdAt) }));
  }
</script>

<a href="/repos/{data.repo.id}">← {data.repo.name}</a>
<h1>Sessions — {data.repo.name}</h1>

{#if data.sessions.length === 0}
  <p>No sessions yet. The MCP server creates one on the first <code>get_session_context</code> call.</p>
{:else}
  <p>{data.sessions.length} session{data.sessions.length !== 1 ? 's' : ''}</p>

  {#each data.sessions as session (session.id)}
    <div class="session">
      <div class="session-header" onclick={() => toggle(session.id)} role="button" tabindex="0"
           onkeydown={(e) => e.key === 'Enter' && toggle(session.id)}>
        <div class="session-title">
          <span class="status status-{session.status}">{session.status}</span>
          <span class="session-id">{session.id.slice(0, 8)}…</span>
        </div>
        <div class="session-meta">
          <span>♥ {fmt(session.lastHeartbeat)}</span>
          <span>{fmt(session.createdAt)}</span>
          <span>{expandedId === session.id ? '▲' : '▼'}</span>
        </div>
      </div>

      {#if expandedId === session.id}
        <div class="session-detail">
          <h3>Snapshots ({session.snapshots.length})</h3>
          {#if session.snapshots.length === 0}
            <p class="empty">No snapshots in this session.</p>
          {:else}
            <Table data={snapshotRows(session.snapshots)} columns={snapshotColumns} />
          {/if}

          <h3>Memory Items ({session.memoryItems.length})</h3>
          {#if session.memoryItems.length === 0}
            <p class="empty">No memory items recorded in this session.</p>
          {:else}
            <Table data={session.memoryItems} columns={memoryColumns} />
          {/if}
        </div>
      {/if}
    </div>
  {/each}
{/if}

<style>
  .session { border: 1px solid #e2e8f0; border-radius: 8px; margin-bottom: 12px; overflow: hidden; }
  .session-header { display: flex; justify-content: space-between; align-items: center; padding: 12px 16px; cursor: pointer; background: #f8fafc; }
  .session-header:hover { background: #f1f5f9; }
  .session-title { display: flex; align-items: center; gap: 12px; }
  .session-meta { display: flex; gap: 16px; color: #64748b; font-size: 0.875rem; }
  .session-detail { padding: 16px; }
  .session-id { font-family: monospace; color: #64748b; font-size: 0.875rem; }
  .status { padding: 2px 8px; border-radius: 12px; font-size: 0.75rem; font-weight: 600; text-transform: uppercase; }
  .status-active { background: #dcfce7; color: #166534; }
  .status-completed { background: #f1f5f9; color: #475569; }
  .status-crashed { background: #fee2e2; color: #991b1b; }
  .empty { color: #94a3b8; font-style: italic; }
  h3 { font-size: 0.875rem; font-weight: 600; color: #374151; margin: 12px 0 8px; }
</style>
```

- [ ] **Step 2: Verify no TypeScript/Svelte errors**

```bash
cd apps/dashboard && bunx svelte-check --tsconfig ./tsconfig.json 2>&1 | head -30
```

Expected: no errors.

- [ ] **Step 3: Commit**

```bash
git add apps/dashboard/src/routes/repos/\[id\]/sessions/+page.svelte
git commit -m "feat(dashboard): add sessions inspector page with status badges and expandable details"
```

---

### Task 14: Add "View Sessions →" Link to Repo Detail Page

**Files:**
- Modify: `apps/dashboard/src/routes/repos/[id]/+page.svelte`

- [ ] **Step 1: Add the link**

In `apps/dashboard/src/routes/repos/[id]/+page.svelte`, after the existing context-packs link:

```svelte
<p><a href="/repos/{data.repo.id}/context-packs">View Context Packs →</a></p>
<p><a href="/repos/{data.repo.id}/sessions">View Sessions →</a></p>
```

- [ ] **Step 2: Verify no TypeScript/Svelte errors**

```bash
cd apps/dashboard && bunx svelte-check --tsconfig ./tsconfig.json 2>&1 | head -30
```

Expected: no errors.

- [ ] **Step 3: Run a full test suite**

```bash
cd /Users/Jerry/Developer/sensei
cd packages/shared && bunx vitest run && cd ../engine && bunx vitest run && cd ../server && bunx vitest run
```

Expected: all tests pass.

- [ ] **Step 4: Commit**

```bash
git add apps/dashboard/src/routes/repos/\[id\]/+page.svelte
git commit -m "feat(dashboard): add View Sessions link to repo detail page"
```

---

## Acceptance Checklist

Before declaring Phase 3 done, verify:

- [ ] `supabase db push` applies the migration without errors
- [ ] `get_session_context` returns `session_id`, `interrupted`, and `memory` fields
- [ ] Calling `get_session_context` twice in the same server process does not create two sessions
- [ ] `take_snapshot` writes a `manual` snapshot and returns its ID
- [ ] `checkpoint` writes a `checkpoint` snapshot and marks the session `completed`
- [ ] `record_memory` creates a `decision` / `pattern` / `question` memory item
- [ ] `close_memory` resolves an open question; calling it again throws "already closed"
- [ ] Hard-killing the MCP server and starting a new session surfaces the crashed session's last snapshot in `interrupted[]`
- [ ] Dashboard `/repos/[id]/sessions` lists sessions with status badges
- [ ] Expanding a session shows its snapshot timeline and memory items
