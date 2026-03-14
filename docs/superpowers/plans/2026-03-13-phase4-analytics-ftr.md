# Analytics & FTR Implementation Plan

> **For agentic workers:** REQUIRED: Use superpowers:subagent-driven-development (if subagents available) or superpowers:executing-plans to implement this plan. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add task session tracking and First-Time-Right (FTR) scoring so developers can measure how effectively agents complete tasks, surfaced via a dashboard analytics view and `sensei stats` CLI.

**Architecture:** Two new DB tables (`task_sessions`, `task_turns`) linked to the existing Phase 3 `sessions` table. FTR is a heuristic 0.0–1.0 score computed at `checkpoint` time from snapshot count, tool error rate, and session completion status. `mcp-server.ts` gains a `taskSessionId` closure alongside `sessionId`; the `beat()` wrapper writes turn rows; `get_session_context` accepts an optional `task_description` to name the task.

**Tech Stack:** TypeScript, Bun, Vitest, Supabase (PostgreSQL), SvelteKit, Rokkit UI

---

## File Structure

| File | Action | Responsibility |
|------|--------|----------------|
| `supabase/migrations/20260313000003_phase4_analytics_ftr.sql` | Create | DB migration: task_sessions + task_turns |
| `database/ddl/table/sensei/task_sessions.ddl` | Create | DDL source for task_sessions |
| `database/ddl/table/sensei/task_turns.ddl` | Create | DDL source for task_turns |
| `packages/engine/src/analytics/ftr.ts` | Create | `computeFtr` (pure), `computeAndStoreFtr` |
| `packages/engine/src/analytics/ftr.spec.ts` | Create | Tests for both FTR functions |
| `packages/engine/src/analytics/task-session.ts` | Create | `createTaskSession`, `recordTaskTurn`, `completeTaskSession`, `getTaskSessions` |
| `packages/engine/src/analytics/task-session.spec.ts` | Create | Tests for all task session functions |
| `packages/engine/src/index.ts` | Modify | Export analytics functions |
| `packages/server/src/mcp-server.ts` | Modify | `taskSessionId` closure, `beat()` extended, `task_description` param, checkpoint handler calls `completeTaskSession` |
| `packages/cli/src/commands/stats.ts` | Modify | Replace implementation to read from Supabase task_sessions + task_turns |
| `packages/cli/src/commands/stats.spec.ts` | Modify | Update tests to match new StatsResult shape |
| `apps/dashboard/src/routes/repos/[id]/analytics/+page.server.ts` | Create | Load task_sessions + task_turns for analytics |
| `apps/dashboard/src/routes/repos/[id]/analytics/+page.svelte` | Create | Tool usage table + session list with FTR |
| `apps/dashboard/src/routes/repos/[id]/+page.svelte` | Modify | Add "View Analytics →" link |

---

## Chunk 1: Database

### Task 1: Migration and DDL Files

**Files:**
- Create: `supabase/migrations/20260313000003_phase4_analytics_ftr.sql`
- Create: `database/ddl/table/sensei/task_sessions.ddl`
- Create: `database/ddl/table/sensei/task_turns.ddl`

No tests for SQL — verify syntax by inspection.

- [ ] **Step 1: Create the migration file**

```sql
-- supabase/migrations/20260313000003_phase4_analytics_ftr.sql

-- Task sessions: one per agent task (checkpoint boundary)
create table if not exists sensei.task_sessions (
  id               uuid        primary key default gen_random_uuid(),
  session_id       uuid        references sensei.sessions(id) on delete set null,
  repo_id          uuid        not null references sensei.repos(id) on delete cascade,
  task_description text,
  task_type        text        check (task_type in ('feat','fix','refactor','docs','test','chore','unknown')),
  status           text        not null default 'in_progress'
                               check (status in ('in_progress','completed','abandoned')),
  ftr_score        numeric(4,3),
  ftr_signals      jsonb,
  created_at       timestamptz not null default now(),
  completed_at     timestamptz
);

create index if not exists task_sessions_repo_id_idx    on sensei.task_sessions(repo_id, created_at desc);
create index if not exists task_sessions_session_id_idx on sensei.task_sessions(session_id)
  where session_id is not null;

-- Task turns: one per post-phase tool call
create table if not exists sensei.task_turns (
  id              uuid        primary key default gen_random_uuid(),
  task_session_id uuid        not null references sensei.task_sessions(id) on delete cascade,
  repo_id         uuid        not null references sensei.repos(id) on delete cascade,
  tool            text        not null,
  success         boolean,
  duration_ms     integer,
  created_at      timestamptz not null default now()
);

create index if not exists task_turns_task_session_id_idx on sensei.task_turns(task_session_id, created_at desc);
create index if not exists task_turns_repo_id_idx         on sensei.task_turns(repo_id, created_at desc);

-- Grants
grant all on all tables in schema sensei to anon, authenticated, service_role;
grant all on all sequences in schema sensei to anon, authenticated, service_role;
grant execute on all functions in schema sensei to anon, authenticated, service_role;
```

- [ ] **Step 2: Create task_sessions DDL source**

```sql
-- database/ddl/table/sensei/task_sessions.ddl
set search_path to sensei, extensions;

create table if not exists task_sessions (
  id               uuid        primary key default gen_random_uuid()
, session_id       uuid        references sensei.sessions(id) on delete set null
, repo_id          uuid        not null references sensei.repos(id) on delete cascade
, task_description text
, task_type        text
                   check (task_type in ('feat','fix','refactor','docs','test','chore','unknown'))
, status           text        not null default 'in_progress'
                   check (status in ('in_progress','completed','abandoned'))
, ftr_score        numeric(4,3)
, ftr_signals      jsonb
, created_at       timestamptz not null default now()
, completed_at     timestamptz
);

create index if not exists task_sessions_repo_id_idx    on task_sessions(repo_id, created_at desc);
create index if not exists task_sessions_session_id_idx on task_sessions(session_id)
  where session_id is not null;

comment on table task_sessions is
'One row per agent task (checkpoint boundary).
- session_id: FK to sessions; nullable on session delete
- task_description: optional task description passed by agent at get_session_context
- task_type: auto-detected from task_description keywords
- ftr_score: 0.000–1.000, null until checkpoint is called
- ftr_signals: raw signals used to compute the score (for auditability)
- status transitions: in_progress → completed (via checkpoint) or abandoned';
```

- [ ] **Step 3: Create task_turns DDL source**

```sql
-- database/ddl/table/sensei/task_turns.ddl
set search_path to sensei, extensions;

create table if not exists task_turns (
  id              uuid        primary key default gen_random_uuid()
, task_session_id uuid        not null references sensei.task_sessions(id) on delete cascade
, repo_id         uuid        not null references sensei.repos(id) on delete cascade
, tool            text        not null
, success         boolean
, duration_ms     integer
, created_at      timestamptz not null default now()
);

create index if not exists task_turns_task_session_id_idx on task_turns(task_session_id, created_at desc);
create index if not exists task_turns_repo_id_idx         on task_turns(repo_id, created_at desc);

comment on table task_turns is
'One row per post-phase tool call within a task session.
- Written by the beat() wrapper in mcp-server.ts after each tool call
- repo_id is denormalized for direct indexed queries without join
- success: null if unknown (pre-phase only), false if tool threw
- Used to compute FTR toolErrorRate signal at checkpoint time';
```

- [ ] **Step 4: Commit**

```bash
git add supabase/migrations/20260313000003_phase4_analytics_ftr.sql \
        database/ddl/table/sensei/task_sessions.ddl \
        database/ddl/table/sensei/task_turns.ddl
git commit -m "feat(db): add task_sessions and task_turns tables for Phase 4 analytics"
```

---

## Chunk 2: FTR Engine

### Task 2: `computeFtr` — Pure FTR Scoring Function

**Files:**
- Create: `packages/engine/src/analytics/ftr.spec.ts` (test first)
- Create: `packages/engine/src/analytics/ftr.ts`

Context: `computeFtr` is a pure function — no DB, no async. It takes `FtrSignals` and returns a number 0.0–1.0. Penalty rules from the spec:
- Extra snapshots (beyond 1): −0.05 each, capped at −0.30
- Tool error rate ≥ 20%: −0.20; ≥ 10%: −0.10
- Not completed cleanly: −0.30
- No task description: cap final score at 0.70
- Result clamped to [0.0, 1.0], rounded to 3 decimal places

- [ ] **Step 1: Write the failing tests**

```typescript
// packages/engine/src/analytics/ftr.spec.ts
import { describe, it, expect } from "vitest";
import { computeFtr } from "./ftr.js";
import type { FtrSignals } from "./ftr.js";

const perfect: FtrSignals = {
  snapshotCount: 1,
  toolErrorRate: 0,
  completedCleanly: true,
  hasDescription: true,
};

describe("computeFtr", () => {
  it("returns 1.0 for perfect signals", () => {
    expect(computeFtr(perfect)).toBe(1.0);
  });

  it("deducts 0.05 per extra snapshot beyond the first", () => {
    expect(computeFtr({ ...perfect, snapshotCount: 2 })).toBe(0.95);
    expect(computeFtr({ ...perfect, snapshotCount: 3 })).toBe(0.9);
    expect(computeFtr({ ...perfect, snapshotCount: 4 })).toBe(0.85);
  });

  it("caps snapshot penalty at -0.30 (7+ snapshots)", () => {
    expect(computeFtr({ ...perfect, snapshotCount: 10 })).toBe(0.7);
    expect(computeFtr({ ...perfect, snapshotCount: 100 })).toBe(0.7);
  });

  it("deducts 0.10 for error rate in [0.10, 0.20)", () => {
    expect(computeFtr({ ...perfect, toolErrorRate: 0.10 })).toBe(0.9);
    expect(computeFtr({ ...perfect, toolErrorRate: 0.19 })).toBe(0.9);
  });

  it("deducts 0.20 for error rate >= 0.20", () => {
    expect(computeFtr({ ...perfect, toolErrorRate: 0.20 })).toBe(0.8);
    expect(computeFtr({ ...perfect, toolErrorRate: 1.0 })).toBe(0.8);
  });

  it("deducts 0.30 when session did not complete cleanly", () => {
    expect(computeFtr({ ...perfect, completedCleanly: false })).toBe(0.7);
  });

  it("caps score at 0.70 when hasDescription is false", () => {
    expect(computeFtr({ ...perfect, hasDescription: false })).toBe(0.7);
    // Even if all other signals are perfect, cap applies
    expect(computeFtr({ ...perfect, hasDescription: false, snapshotCount: 2 })).toBe(0.7);
  });

  it("maximum penalties stack to yield 0.20 (the mathematical floor)", () => {
    // Maximum possible penalty: -0.30 (snap cap) + -0.20 (error) + -0.30 (crash) = -0.80
    // 1.0 - 0.80 = 0.20 is the lowest reachable score before the no-description cap
    const worst: FtrSignals = {
      snapshotCount: 10,       // -0.30 (capped)
      toolErrorRate: 0.5,      // -0.20
      completedCleanly: false, // -0.30
      hasDescription: true,
    };
    expect(computeFtr(worst)).toBe(0.2);
  });


  it("hasDescription=false cap applies after other penalties", () => {
    // 1.0 - 0.05 (1 extra snap) = 0.95, then capped at 0.70
    expect(computeFtr({ ...perfect, snapshotCount: 2, hasDescription: false })).toBe(0.7);
    // 1.0 - 0.30 (no clean) = 0.70, then capped at min(0.70, 0.70) = 0.70
    expect(computeFtr({ ...perfect, completedCleanly: false, hasDescription: false })).toBe(0.7);
    // 1.0 - 0.30 - 0.30 = 0.40, then capped at min(0.40, 0.70) = 0.40
    expect(computeFtr({ ...perfect, snapshotCount: 10, completedCleanly: false, hasDescription: false })).toBe(0.4);
  });
});
```

- [ ] **Step 2: Run tests — expect failure (ftr.ts does not exist)**

```bash
cd packages/engine && bunx vitest run src/analytics/ftr.spec.ts
```

Expected: FAIL with "Cannot find module './ftr.js'"

- [ ] **Step 3: Implement `computeFtr`**

```typescript
// packages/engine/src/analytics/ftr.ts
import type { SupabaseClient } from "@supabase/supabase-js";

export interface FtrSignals {
  snapshotCount: number;
  toolErrorRate: number;     // 0.0–1.0
  completedCleanly: boolean;
  hasDescription: boolean;
}

export interface FtrResult {
  score: number;             // 0.000–1.000
  signals: FtrSignals;
}

export function computeFtr(signals: FtrSignals): number {
  let score = 1.0;

  // Snapshot penalty: -0.05 per snapshot beyond the first, capped at -0.30
  const extraSnapshots = Math.max(0, signals.snapshotCount - 1);
  score -= Math.min(extraSnapshots * 0.05, 0.30);

  // Tool error rate penalty
  if (signals.toolErrorRate >= 0.20) {
    score -= 0.20;
  } else if (signals.toolErrorRate >= 0.10) {
    score -= 0.10;
  }

  // Session completion penalty
  if (!signals.completedCleanly) {
    score -= 0.30;
  }

  // No description cap (applied after all penalties)
  if (!signals.hasDescription) {
    score = Math.min(score, 0.70);
  }

  // Clamp to [0.0, 1.0] and round to 3 decimal places
  return Math.round(Math.max(0, Math.min(1, score)) * 1000) / 1000;
}
```

- [ ] **Step 4: Run tests — expect all pass**

```bash
cd packages/engine && bunx vitest run src/analytics/ftr.spec.ts
```

Expected: All 9 tests PASS

- [ ] **Step 5: Commit**

```bash
git add packages/engine/src/analytics/ftr.ts packages/engine/src/analytics/ftr.spec.ts
git commit -m "feat(engine): add computeFtr pure function with full test coverage"
```

---

### Task 3: `computeAndStoreFtr` — DB-Backed FTR Computation

**Files:**
- Modify: `packages/engine/src/analytics/ftr.ts` (add function)
- Modify: `packages/engine/src/analytics/ftr.spec.ts` (add tests)

Context: `computeAndStoreFtr` fetches signals from DB (snapshots count, task_turns error rate, session status, task_description presence), calls `computeFtr`, and updates the `task_sessions` row. It throws on any DB error.

The DB calls in order:
1. `db.from("snapshots").select("*", {count:"exact", head:true}).eq("session_id", sessionId)` → snapshotCount
2. `db.from("task_turns").select("success").eq("task_session_id", taskSessionId)` → turns array
3. `db.from("sessions").select("status").eq("id", sessionId).single()` → session status
4. `db.from("task_sessions").select("task_description").eq("id", taskSessionId).single()` → description
5. `db.from("task_sessions").update({ftr_score, ftr_signals}).eq("id", taskSessionId)` → store result

- [ ] **Step 1: Add tests for `computeAndStoreFtr`**

Add the following block to `packages/engine/src/analytics/ftr.spec.ts`:

```typescript
import { computeFtr, computeAndStoreFtr } from "./ftr.js";

// ... existing tests above ...

describe("computeAndStoreFtr", () => {
  function makeDb(overrides: {
    snapshotCount?: number;
    turns?: Array<{ success: boolean | null }>;
    sessionStatus?: string;
    taskDescription?: string | null;
    updateError?: string | null;
  } = {}) {
    const {
      snapshotCount = 1,
      turns = [],
      sessionStatus = "completed",
      taskDescription = "add new feature",
      updateError = null,
    } = overrides;

    // Disambiguate the two task_sessions calls by their own counter
    let taskSessionsCallCount = 0;
    return {
      from: vi.fn().mockImplementation((table: string) => {
        if (table === "snapshots") {
          return {
            select: vi.fn().mockReturnThis(),
            eq: vi.fn().mockResolvedValue({ count: snapshotCount, error: null }),
          };
        }
        if (table === "task_turns") {
          return {
            select: vi.fn().mockReturnThis(),
            eq: vi.fn().mockResolvedValue({ data: turns, error: null }),
          };
        }
        if (table === "sessions") {
          return {
            select: vi.fn().mockReturnThis(),
            eq: vi.fn().mockReturnThis(),
            single: vi.fn().mockResolvedValue({ data: { status: sessionStatus }, error: null }),
          };
        }
        // task_sessions: first call = select task_description, second = update ftr_score
        taskSessionsCallCount++;
        if (taskSessionsCallCount === 1) {
          return {
            select: vi.fn().mockReturnThis(),
            eq: vi.fn().mockReturnThis(),
            single: vi.fn().mockResolvedValue({ data: { task_description: taskDescription }, error: null }),
          };
        }
        return {
          update: vi.fn().mockReturnValue({
            eq: vi.fn().mockResolvedValue({ error: updateError ? { message: updateError } : null }),
          }),
        };
      }),
    } as any;
  }

  it("computes FTR from DB signals and stores result", async () => {
    const db = makeDb({ snapshotCount: 2, turns: [{ success: true }, { success: true }] });
    const result = await computeAndStoreFtr(db, "ts-1", "sess-1");
    // snapshotCount=2 → -0.05, no errors, clean, hasDescription → 0.95
    expect(result.score).toBe(0.95);
    expect(result.signals.snapshotCount).toBe(2);
    expect(result.signals.toolErrorRate).toBe(0);
    expect(result.signals.completedCleanly).toBe(true);
    expect(result.signals.hasDescription).toBe(true);
  });

  it("sets hasDescription=false when task_description is null", async () => {
    const db = makeDb({ taskDescription: null });
    const result = await computeAndStoreFtr(db, "ts-1", "sess-1");
    expect(result.signals.hasDescription).toBe(false);
    expect(result.score).toBeLessThanOrEqual(0.7);
  });

  it("sets completedCleanly=false when session status is not completed", async () => {
    const db = makeDb({ sessionStatus: "crashed" });
    const result = await computeAndStoreFtr(db, "ts-1", "sess-1");
    expect(result.signals.completedCleanly).toBe(false);
    expect(result.score).toBeLessThanOrEqual(0.7);
  });

  it("throws on DB update error", async () => {
    const db = makeDb({ updateError: "write failed" });
    await expect(computeAndStoreFtr(db, "ts-1", "sess-1")).rejects.toThrow("write failed");
  });

  it("zero-turn session gets toolErrorRate=0 (best-case default)", async () => {
    const db = makeDb({ turns: [] });
    const result = await computeAndStoreFtr(db, "ts-1", "sess-1");
    expect(result.signals.toolErrorRate).toBe(0);
    // Perfect signals (snapshotCount=1, no errors, clean, hasDescription) → 1.0
    expect(result.score).toBe(1.0);
  });

  it("throws on DB read error (snapshots)", async () => {
    const db = {
      from: vi.fn().mockImplementation((table: string) => {
        if (table === "snapshots") {
          return { select: vi.fn().mockReturnThis(), eq: vi.fn().mockResolvedValue({ count: null, error: { message: "snapshots unavailable" } }) };
        }
        // Other tables never reached
        return { select: vi.fn().mockReturnThis(), eq: vi.fn().mockReturnThis(), single: vi.fn().mockResolvedValue({ data: null, error: null }) };
      }),
    } as any;
    await expect(computeAndStoreFtr(db, "ts-1", "sess-1")).rejects.toThrow("snapshots unavailable");
  });
});
```

- [ ] **Step 2: Run tests — expect new tests to fail**

```bash
cd packages/engine && bunx vitest run src/analytics/ftr.spec.ts
```

Expected: `computeAndStoreFtr` tests FAIL with "not a function"

- [ ] **Step 3: Implement `computeAndStoreFtr`**

Add to `packages/engine/src/analytics/ftr.ts` after `computeFtr`:

```typescript
export async function computeAndStoreFtr(
  db: SupabaseClient,
  taskSessionId: string,
  sessionId: string,
): Promise<FtrResult> {
  // 1. Snapshot count (all kinds: manual + checkpoint)
  const { count: snapshotCount, error: snapErr } = await db
    .from("snapshots")
    .select("*", { count: "exact", head: true })
    .eq("session_id", sessionId);
  if (snapErr) throw new Error((snapErr as { message?: string }).message ?? "Failed to fetch snapshot count");

  // 2. Task turns error rate
  const { data: turns, error: turnsErr } = await db
    .from("task_turns")
    .select("success")
    .eq("task_session_id", taskSessionId);
  if (turnsErr) throw new Error((turnsErr as { message?: string }).message ?? "Failed to fetch task turns");
  const allTurns = (turns ?? []) as Array<{ success: boolean | null }>;
  const totalTurns = allTurns.length;
  const errorCount = allTurns.filter(t => t.success === false).length;
  // Zero turns: no errors recorded → toolErrorRate = 0 (best-case default)
  const toolErrorRate = totalTurns > 0 ? errorCount / totalTurns : 0;

  // 3. Session completion status; completedCleanly = status === 'completed'
  //    (checkpointTool sets status='completed' before this is called on the clean path)
  const { data: session, error: sessErr } = await db
    .from("sessions")
    .select("status")
    .eq("id", sessionId)
    .single();
  if (sessErr) throw new Error((sessErr as { message?: string }).message ?? "Failed to fetch session status");

  // 4. Task description presence
  const { data: taskSession, error: tsErr } = await db
    .from("task_sessions")
    .select("task_description")
    .eq("id", taskSessionId)
    .single();
  if (tsErr) throw new Error((tsErr as { message?: string }).message ?? "Failed to fetch task session");

  const signals: FtrSignals = {
    snapshotCount: snapshotCount ?? 0,
    toolErrorRate,
    completedCleanly: (session as Record<string, unknown> | null)?.status === "completed",
    hasDescription: !!(taskSession as Record<string, unknown> | null)?.task_description,
  };

  const score = computeFtr(signals);

  const { error } = await db
    .from("task_sessions")
    .update({ ftr_score: score, ftr_signals: signals })
    .eq("id", taskSessionId);

  if (error) throw new Error((error as { message?: string }).message ?? "Failed to store FTR");

  return { score, signals };
}
```

- [ ] **Step 4: Run all FTR tests — expect all pass**

```bash
cd packages/engine && bunx vitest run src/analytics/ftr.spec.ts
```

Expected: All tests PASS

- [ ] **Step 5: Commit**

```bash
git add packages/engine/src/analytics/ftr.ts packages/engine/src/analytics/ftr.spec.ts
git commit -m "feat(engine): add computeAndStoreFtr with DB signal fetching"
```

---

## Chunk 3: Task Session Engine

### Task 4: `createTaskSession` and `recordTaskTurn`

**Files:**
- Create: `packages/engine/src/analytics/task-session.spec.ts` (test first)
- Create: `packages/engine/src/analytics/task-session.ts`

Context: `createTaskSession` inserts a `task_sessions` row, auto-detecting `task_type` from keywords. `recordTaskTurn` inserts a `task_turns` row silently (never throws).

Keyword detection rules:
- `fix|bug|patch|broken|error` → `'fix'`
- `feat|add|implement|build|create|new` → `'feat'`
- `refactor|clean|extract|restructure` → `'refactor'`
- `docs?|document|readme|comment` → `'docs'`
- `test|spec|coverage` → `'test'`
- `chore|bump|update|upgrade|dep` → `'chore'`
- else → `'unknown'`
- no description → `'unknown'`

- [ ] **Step 1: Write failing tests**

```typescript
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
    // Verify insert was called with task_type: 'refactor'
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
```

- [ ] **Step 2: Run tests — expect failure**

```bash
cd packages/engine && bunx vitest run src/analytics/task-session.spec.ts
```

Expected: FAIL with "Cannot find module './task-session.js'"

- [ ] **Step 3: Implement `createTaskSession` and `recordTaskTurn`**

```typescript
// packages/engine/src/analytics/task-session.ts
import type { SupabaseClient } from "@supabase/supabase-js";
import { computeAndStoreFtr } from "./ftr.js";
import type { FtrResult, FtrSignals } from "./ftr.js";

export type TaskType = 'feat' | 'fix' | 'refactor' | 'docs' | 'test' | 'chore' | 'unknown';
export type TaskStatus = 'in_progress' | 'completed' | 'abandoned';

export interface TaskSessionInfo {
  id: string;
  taskType: TaskType | null;
  createdAt: string;
}

export interface TaskSession {
  id: string;
  sessionId: string | null;
  repoId: string;
  taskDescription: string | null;
  taskType: TaskType | null;
  status: TaskStatus;
  ftrScore: number | null;
  ftrSignals: FtrSignals | null;
  createdAt: string;
  completedAt: string | null;
}

function detectTaskType(description: string): TaskType {
  const lower = description.toLowerCase();
  if (/fix|bug|patch|broken|error/.test(lower)) return 'fix';
  if (/feat|add|implement|build|create|new/.test(lower)) return 'feat';
  if (/refactor|clean|extract|restructure/.test(lower)) return 'refactor';
  if (/docs?|document|readme|comment/.test(lower)) return 'docs';
  if (/test|spec|coverage/.test(lower)) return 'test';
  if (/chore|bump|update|upgrade|dep/.test(lower)) return 'chore';
  return 'unknown';
}

export async function createTaskSession(
  db: SupabaseClient,
  sessionId: string,
  repoId: string,
  taskDescription?: string,
): Promise<TaskSessionInfo> {
  const taskType: TaskType = taskDescription ? detectTaskType(taskDescription) : 'unknown';

  const { data, error } = await db
    .from("task_sessions")
    .insert({
      session_id: sessionId,
      repo_id: repoId,
      task_description: taskDescription ?? null,
      task_type: taskType,
    })
    .select()
    .single();

  if (error || !data) throw new Error((error as { message?: string } | null)?.message ?? "Failed to create task session");

  return {
    id: (data as Record<string, unknown>).id as string,
    taskType: (data as Record<string, unknown>).task_type as TaskType | null,
    createdAt: (data as Record<string, unknown>).created_at as string,
  };
}

export async function recordTaskTurn(
  db: SupabaseClient,
  taskSessionId: string,
  repoId: string,
  tool: string,
  success: boolean | null,
  durationMs?: number | null,
): Promise<void> {
  try {
    await db.from("task_turns").insert({
      task_session_id: taskSessionId,
      repo_id: repoId,
      tool,
      success,
      duration_ms: durationMs ?? null,
    });
  } catch {
    // silent best-effort — a missed turn row widens error rate estimate but never blocks
  }
}
```

- [ ] **Step 4: Run tests — expect all pass**

```bash
cd packages/engine && bunx vitest run src/analytics/task-session.spec.ts
```

Expected: All tests PASS

- [ ] **Step 5: Commit**

```bash
git add packages/engine/src/analytics/task-session.ts packages/engine/src/analytics/task-session.spec.ts
git commit -m "feat(engine): add createTaskSession and recordTaskTurn"
```

---

### Task 5: `completeTaskSession` and `getTaskSessions`

**Files:**
- Modify: `packages/engine/src/analytics/task-session.ts`
- Modify: `packages/engine/src/analytics/task-session.spec.ts`

Context: `completeTaskSession` calls `computeAndStoreFtr`, then sets `status='completed'` + `completed_at`. `getTaskSessions` returns all task sessions for a repo within N days, never throws.

- [ ] **Step 1: Add tests for `completeTaskSession` and `getTaskSessions`**

Add to `packages/engine/src/analytics/task-session.spec.ts`:

```typescript
import { createTaskSession, recordTaskTurn, completeTaskSession, getTaskSessions } from "./task-session.js";

// ... existing tests above ...

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
```

- [ ] **Step 2: Run tests — expect new tests to fail**

```bash
cd packages/engine && bunx vitest run src/analytics/task-session.spec.ts
```

Expected: `completeTaskSession` and `getTaskSessions` FAIL with "not a function"

- [ ] **Step 3: Implement `completeTaskSession` and `getTaskSessions`**

Add to `packages/engine/src/analytics/task-session.ts`:

```typescript
export async function completeTaskSession(
  db: SupabaseClient,
  taskSessionId: string,
  sessionId: string,
): Promise<FtrResult> {
  // Compute and store FTR score (reads DB signals, writes ftr_score + ftr_signals)
  const ftrResult = await computeAndStoreFtr(db, taskSessionId, sessionId);

  // Mark task session completed
  const { error } = await db
    .from("task_sessions")
    .update({ status: "completed", completed_at: new Date().toISOString() })
    .eq("id", taskSessionId);

  if (error) throw new Error((error as { message?: string }).message ?? "Failed to complete task session");

  return ftrResult;
}

export async function getTaskSessions(
  db: SupabaseClient,
  repoId: string,
  limitDays = 30,
): Promise<TaskSession[]> {
  try {
    const since = new Date(Date.now() - limitDays * 24 * 60 * 60 * 1000).toISOString();

    const { data, error } = await db
      .from("task_sessions")
      .select("*")
      .eq("repo_id", repoId)
      .gte("created_at", since)
      .order("created_at", { ascending: false });

    if (error || !data) return [];

    return (data as Array<Record<string, unknown>>).map(row => ({
      id: row.id as string,
      sessionId: (row.session_id as string | null) ?? null,
      repoId: row.repo_id as string,
      taskDescription: (row.task_description as string | null) ?? null,
      taskType: (row.task_type as TaskType | null) ?? null,
      status: row.status as TaskStatus,
      ftrScore: (row.ftr_score as number | null) ?? null,
      ftrSignals: (row.ftr_signals as FtrSignals | null) ?? null,
      createdAt: row.created_at as string,
      completedAt: (row.completed_at as string | null) ?? null,
    }));
  } catch {
    return [];
  }
}
```

- [ ] **Step 4: Run all task-session tests — expect all pass**

```bash
cd packages/engine && bunx vitest run src/analytics/task-session.spec.ts
```

Expected: All tests PASS

- [ ] **Step 5: Run full engine test suite — expect no regressions**

```bash
cd packages/engine && bunx vitest run
```

Expected: All tests PASS

- [ ] **Step 6: Commit**

```bash
git add packages/engine/src/analytics/task-session.ts packages/engine/src/analytics/task-session.spec.ts
git commit -m "feat(engine): add completeTaskSession and getTaskSessions"
```

---

### Task 6: Export Analytics from Engine Index

**Files:**
- Modify: `packages/engine/src/index.ts`

- [ ] **Step 1: Add analytics exports**

In `packages/engine/src/index.ts`, add two lines at the end:

```typescript
export * from "./analytics/ftr.js";
export * from "./analytics/task-session.js";
```

The file should now end with:
```typescript
export * from "./session/session-manager.js";
export * from "./session/snapshot.js";
export * from "./session/memory.js";
export * from "./analytics/ftr.js";
export * from "./analytics/task-session.js";
```

- [ ] **Step 2: Verify imports resolve**

```bash
cd packages/engine && bunx vitest run
```

Expected: All tests PASS (no import errors)

- [ ] **Step 3: Commit**

```bash
git add packages/engine/src/index.ts
git commit -m "feat(engine): export analytics functions from package index"
```

---

## Chunk 4: MCP Layer

### Task 7: `mcp-server.ts` — Task Session Integration

**Files:**
- Modify: `packages/server/src/mcp-server.ts`

Context: Three changes to `mcp-server.ts`:
1. Add `taskSessionId: string | null = null` closure variable alongside `sessionId`
2. Change `beat(client)` signature to `beat(client, toolName, success)` and extend it to call `recordTaskTurn`
3. Add `task_description` optional param to `get_session_context` tool; call `createTaskSession` after session creation
4. In the `checkpoint` handler, call `completeTaskSession` after `checkpointTool` returns

The existing `beat(client)` is called at the end of every tool handler. After this change, every call site becomes `beat(client, "tool_name", true)`.

- [ ] **Step 1: Add imports**

At the top of `packages/server/src/mcp-server.ts`, add to the `@sensei/engine` import:

```typescript
import { createSession, updateHeartbeat, createTaskSession, recordTaskTurn, completeTaskSession } from "@sensei/engine";
```

(Replace the existing `import { createSession, updateHeartbeat } from "@sensei/engine";`)

- [ ] **Step 2: Add `taskSessionId` closure and update `beat()`**

Replace the existing closure + beat declaration:
```typescript
// Session state — stored per MCP server process
let sessionId: string | null = null;

const beat = (client: any) => {
  if (sessionId) updateHeartbeat(client, sessionId).catch(() => {});
};
```

With:
```typescript
// Session state — stored per MCP server process
let sessionId: string | null = null;
let taskSessionId: string | null = null;

const beat = (client: any, toolName: string, success: boolean) => {
  if (sessionId) updateHeartbeat(client, sessionId).catch(() => {});
  if (taskSessionId) recordTaskTurn(client as any, taskSessionId, opts.repoId, toolName, success).catch(() => {});
};
```

- [ ] **Step 3: Update `get_session_context` tool — add `task_description` param and `createTaskSession` call**

Replace the `get_session_context` tool registration (currently has `{}` as schema) with:

```typescript
server.tool(
  "get_session_context",
  "Get orientation context for the current repo — symbol count, stack, last indexed timestamp, interrupted sessions, and project memory",
  {
    task_description: z.string().optional().describe("Brief description of the task you are about to work on — used for FTR tracking"),
  },
  async ({ task_description }) => {
    try {
      const client = await getClient();
      if (!client) return { content: [{ type: "text", text: "Error: Supabase client not configured. Run sensei init first." }] };

      // Create session on first call; reuse on subsequent calls (idempotent)
      // Assign sessionId and taskSessionId atomically — both or neither
      if (!sessionId) {
        const session = await createSession(client as any, opts.repoId);
        const taskSession = await createTaskSession(client as any, session.id, opts.repoId, task_description);
        sessionId = session.id;
        taskSessionId = taskSession.id;
      }

      const result = await getSessionContext(client as any, opts.repoId, opts.repoPath, sessionId);
      beat(client, "get_session_context", true);
      return { content: [{ type: "text", text: JSON.stringify(result, null, 2) }] };
    } catch (err) {
      return { content: [{ type: "text", text: `Error: ${err instanceof Error ? err.message : String(err)}` }], isError: true };
    }
  }
);
```

- [ ] **Step 4: Update `checkpoint` handler — call `completeTaskSession` after `checkpointTool`**

Replace the existing `checkpoint` handler body:

```typescript
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
      // completeTaskSession runs after checkpointTool so sessions.status is already 'completed'
      // Throws on DB error — the outer try/catch surfaces it to the agent as isError: true
      if (taskSessionId) {
        await completeTaskSession(client as any, taskSessionId, sessionId);
      }
      beat(client, "checkpoint", true);
      return { content: [{ type: "text", text: JSON.stringify(result, null, 2) }] };
    } catch (err) {
      return { content: [{ type: "text", text: `Error: ${err instanceof Error ? err.message : String(err)}` }], isError: true };
    }
  }
);
```

- [ ] **Step 5: Update all other `beat(client)` call sites**

Every other tool handler currently calls `beat(client)`. Change each to pass the tool name and `true` for success. The handlers to update:

- `search`: `beat(client)` → `beat(client, "search", true)`
- `load_context`: `beat(client)` → `beat(client, "load_context", true)`
- `context_pack`: `beat(client)` → `beat(client, "context_pack", true)`
- `recommend_next`: `beat(client)` → `beat(client, "recommend_next", true)`
- `token_stats`: `beat(client)` → `beat(client, "token_stats", true)`
- `take_snapshot`: `beat(client)` → `beat(client, "take_snapshot", true)`
- `record_memory`: `beat(client)` → `beat(client, "record_memory", true)`
- `close_memory`: `beat(client)` → `beat(client, "close_memory", true)`

Each error path in a try/catch should pass `false` — but since the current pattern only calls `beat` in the success path (before the error return), the success path only needs `true`. Do not add `beat` calls to error paths.

- [ ] **Step 6: Verify TypeScript compiles**

```bash
cd packages/server && bunx tsc --noEmit
```

Expected: No errors

- [ ] **Step 7: Commit**

```bash
git add packages/server/src/mcp-server.ts
git commit -m "feat(server): add taskSessionId closure, task_description param, and FTR integration to checkpoint"
```

---

## Chunk 5: CLI and Dashboard

### Task 8: Replace `sensei stats` Implementation

**Files:**
- Modify: `packages/cli/src/commands/stats.ts`
- Modify: `packages/cli/src/commands/stats.spec.ts`

Context: The existing `stats.ts` imports `queryStats` and `StatsResult` from `@sensei/collector`. The new implementation reads from Supabase `task_sessions` + `task_turns`. The `--gaps` flag (reads from `events` table) is kept unchanged.

New `StatsResult` shape:
```typescript
interface StatsResult {
  period: { from: string; to: string };
  sessions: { total: number; completed: number; abandoned: number; inProgress: number };
  avgFtr: number | null;
  topTools: Array<{ name: string; calls: number; successRate: number; avgDurationMs: number | null }>;
  errorCount: number;
  errorSessions: number;
}
```

- [ ] **Step 1: Write failing tests**

Replace `packages/cli/src/commands/stats.spec.ts` entirely:

```typescript
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
```

- [ ] **Step 2: Run tests — expect failures**

```bash
cd packages/cli && bunx vitest run src/commands/stats.spec.ts
```

Expected: FAIL — `buildStatsResult` and new `StatsResult` shape don't exist yet

- [ ] **Step 3: Implement new `stats.ts`**

Replace `packages/cli/src/commands/stats.ts` entirely:

```typescript
// packages/cli/src/commands/stats.ts
import { makeSenseiClient } from "@sensei/shared";
import { detectGapPatterns } from "@sensei/collector";

export interface StatsResult {
  period: { from: string; to: string };
  sessions: { total: number; completed: number; abandoned: number; inProgress: number };
  avgFtr: number | null;
  topTools: Array<{ name: string; calls: number; successRate: number; avgDurationMs: number | null }>;
  errorCount: number;
  errorSessions: number;
}

export interface StatsCommandOptions {
  days?: number;
  json?: boolean;
  gaps?: boolean;
  /** Override repo path — used in tests */
  _repoPath?: string;
}

export function buildStatsResult(
  sessions: Array<{ status: string; ftr_score: number | null; id: string }>,
  turns: Array<{ tool: string; success: boolean | null; duration_ms: number | null; task_session_id?: string | null }>,
  period: { from: string; to: string },
): StatsResult {
  const completed = sessions.filter(s => s.status === "completed");
  const abandoned = sessions.filter(s => s.status === "abandoned").length;
  const inProgress = sessions.filter(s => s.status === "in_progress").length;

  const ftrScores = completed.map(s => s.ftr_score).filter((s): s is number => s !== null);
  const avgFtr = ftrScores.length > 0
    ? Math.round((ftrScores.reduce((a, b) => a + b, 0) / ftrScores.length) * 1000) / 1000
    : null;

  // Aggregate turns by tool
  const toolMap = new Map<string, { calls: number; successes: number; totalDuration: number; durationCount: number }>();
  for (const turn of turns) {
    const entry = toolMap.get(turn.tool) ?? { calls: 0, successes: 0, totalDuration: 0, durationCount: 0 };
    entry.calls++;
    if (turn.success === true) entry.successes++;
    if (turn.duration_ms !== null) { entry.totalDuration += turn.duration_ms; entry.durationCount++; }
    toolMap.set(turn.tool, entry);
  }

  const topTools = Array.from(toolMap.entries())
    .map(([name, t]) => ({
      name,
      calls: t.calls,
      successRate: t.calls > 0 ? Math.round((t.successes / t.calls) * 100) / 100 : 0,
      avgDurationMs: t.durationCount > 0 ? Math.round(t.totalDuration / t.durationCount) : null,
    }))
    .sort((a, b) => b.calls - a.calls)
    .slice(0, 10);

  const errorCount = turns.filter(t => t.success === false).length;
  const errorSessionIds = new Set(
    turns
      .filter(t => t.success === false && t.task_session_id != null)
      .map(t => t.task_session_id)
  );

  return {
    period,
    sessions: { total: sessions.length, completed: completed.length, abandoned, inProgress },
    avgFtr,
    topTools,
    errorCount,
    errorSessions: errorSessionIds.size,
  };
}

export function formatStats(result: StatsResult, opts: { json: boolean }): string {
  if (opts.json) return JSON.stringify(result, null, 2);

  const lines: string[] = [];
  const { sessions, avgFtr, topTools, errorCount } = result;

  lines.push(`\nsensei stats — ${result.period.from} → ${result.period.to}`);
  lines.push(`\nSessions  ${sessions.total}   (${sessions.completed} completed, ${sessions.abandoned} abandoned, ${sessions.inProgress} in_progress)`);
  lines.push(`Avg FTR   ${avgFtr !== null ? avgFtr.toFixed(3) : "—"}`);

  if (topTools.length > 0) {
    lines.push(`\nTop tools:`);
    for (const t of topTools) {
      const dur = t.avgDurationMs !== null
        ? t.avgDurationMs >= 1000 ? `${(t.avgDurationMs / 1000).toFixed(1)}s` : `${t.avgDurationMs}ms`
        : "—";
      lines.push(`  ${t.name.padEnd(22)} ${String(t.calls).padStart(4)} calls  ${Math.round(t.successRate * 100)}% success  avg ${dur}`);
    }
  }

  lines.push(`\nErrors    ${errorCount} tool failures`);
  return lines.join("\n");
}

export async function stats(opts: StatsCommandOptions): Promise<void> {
  const client = await makeSenseiClient(opts._repoPath ?? process.cwd());
  if (!client) {
    console.error("Supabase not configured. Run `sensei init` to configure.");
    return;
  }

  const days = opts.days ?? 7;
  const since = new Date(Date.now() - days * 24 * 60 * 60 * 1000).toISOString();
  const to = new Date().toISOString().slice(0, 10);
  const from = new Date(Date.now() - days * 24 * 60 * 60 * 1000).toISOString().slice(0, 10);

  if (opts.gaps) {
    let query = (client as any).from("events").select("input").eq("tool", "Bash").eq("phase", "pre").not("input", "is", null).gte("ts", since);
    const { data: bashEvents } = await query;
    const commands = (bashEvents ?? [])
      .map((e: any) => (e.input as { command?: string })?.command ?? "")
      .filter(Boolean);
    const gaps = detectGapPatterns(commands);
    if (opts.json) {
      console.log(JSON.stringify({ gaps }, null, 2));
    } else {
      console.log(`\nMissed opportunity report — last ${days} days\n`);
      if (gaps.length === 0) { console.log("  No gaps detected."); return; }
      console.log("Pattern                          Count   Suggested tool");
      console.log("─".repeat(60));
      for (const g of gaps) {
        console.log(`${g.pattern.padEnd(32)} ${String(g.count).padStart(5)}   ${g.suggested_tool}`);
      }
    }
    return;
  }

  const { data: sessions } = await (client as any)
    .from("task_sessions")
    .select("id,status,ftr_score")
    .gte("created_at", since);

  const { data: turns } = await (client as any)
    .from("task_turns")
    .select("tool,success,duration_ms,task_session_id")
    .gte("created_at", since);

  const result = buildStatsResult(sessions ?? [], turns ?? [], { from, to });
  console.log(formatStats(result, { json: opts.json ?? false }));
}
```

- [ ] **Step 4: Run tests — expect all pass**

```bash
cd packages/cli && bunx vitest run src/commands/stats.spec.ts
```

Expected: All tests PASS

- [ ] **Step 5: Commit**

```bash
git add packages/cli/src/commands/stats.ts packages/cli/src/commands/stats.spec.ts
git commit -m "feat(cli): replace sensei stats to read from task_sessions and task_turns"
```

---

### Task 9: Dashboard Analytics Route — Server Load

**Files:**
- Create: `apps/dashboard/src/routes/repos/[id]/analytics/+page.server.ts`

Context: Follows the same pattern as `sessions/+page.server.ts`. Returns `repo`, `sessions` (TaskSession array), and `toolUsage` (aggregated task_turns). The `task_turns` query uses the `repo_id` index directly — no join needed.

- [ ] **Step 1: Create server load**

```typescript
// apps/dashboard/src/routes/repos/[id]/analytics/+page.server.ts
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

  const since = new Date(Date.now() - 30 * 24 * 60 * 60 * 1000).toISOString();

  const { data: rawSessions } = await db
    .from('task_sessions')
    .select('id,session_id,task_description,task_type,status,ftr_score,ftr_signals,created_at,completed_at')
    .eq('repo_id', params.id)
    .gte('created_at', since)
    .order('created_at', { ascending: false })
    .limit(100);

  const { data: rawTurns } = await db
    .from('task_turns')
    .select('tool,success,duration_ms,task_session_id')
    .eq('repo_id', params.id)
    .gte('created_at', since);

  // Aggregate turns by tool client-side
  const toolMap = new Map<string, { calls: number; successes: number; totalDuration: number; durationCount: number }>();
  for (const turn of ((rawTurns ?? []) as Array<{ tool: string; success: boolean | null; duration_ms: number | null }>)) {
    const entry = toolMap.get(turn.tool) ?? { calls: 0, successes: 0, totalDuration: 0, durationCount: 0 };
    entry.calls++;
    if (turn.success === true) entry.successes++;
    if (turn.duration_ms !== null) { entry.totalDuration += turn.duration_ms; entry.durationCount++; }
    toolMap.set(turn.tool, entry);
  }

  const toolUsage = Array.from(toolMap.entries())
    .map(([tool, t]) => ({
      tool,
      calls: t.calls,
      successRate: t.calls > 0 ? Math.round((t.successes / t.calls) * 1000) / 1000 : 0,
      avgDurationMs: t.durationCount > 0 ? Math.round(t.totalDuration / t.durationCount) : null,
    }))
    .sort((a, b) => b.calls - a.calls)
    .slice(0, 10);

  const sessions = ((rawSessions ?? []) as Array<Record<string, unknown>>).map(s => ({
    id: s.id as string,
    sessionId: (s.session_id as string | null) ?? null,
    taskDescription: (s.task_description as string | null) ?? null,
    taskType: (s.task_type as string | null) ?? null,
    status: s.status as string,
    ftrScore: (s.ftr_score as number | null) ?? null,
    ftrSignals: (s.ftr_signals as Record<string, unknown> | null) ?? null,
    createdAt: s.created_at as string,
    completedAt: (s.completed_at as string | null) ?? null,
  }));

  return {
    repo: repo as { id: string; name: string },
    sessions,
    toolUsage,
  };
};
```

- [ ] **Step 2: Verify TypeScript compiles**

```bash
cd apps/dashboard && bunx tsc --noEmit
```

Expected: No errors

- [ ] **Step 3: Commit**

```bash
git add apps/dashboard/src/routes/repos/[id]/analytics/+page.server.ts
git commit -m "feat(dashboard): add analytics server load for task_sessions and task_turns"
```

---

### Task 10: Dashboard Analytics Route — Svelte Page + Navigation Link

**Files:**
- Create: `apps/dashboard/src/routes/repos/[id]/analytics/+page.svelte`
- Modify: `apps/dashboard/src/routes/repos/[id]/+page.svelte`

Context: Follows the same style as `sessions/+page.svelte`. Uses Rokkit `Table`, Svelte 5 `$props()`. FTR score is color-coded: ≥0.8 green, 0.5–0.79 yellow, <0.5 red, null grey.

- [ ] **Step 1: Create the analytics Svelte page**

```svelte
<!-- apps/dashboard/src/routes/repos/[id]/analytics/+page.svelte -->
<script lang="ts">
  import { Table } from '@rokkit/ui';
  import type { PageData } from './$types';

  const { data } = $props();

  const fmt = (iso: string) => new Date(iso).toLocaleString();
  const truncate = (s: string | null, n = 60) => s ? (s.length > n ? s.slice(0, n) + '…' : s) : '—';
  const ftrColor = (score: number | null) => {
    if (score === null) return 'ftr-null';
    if (score >= 0.8) return 'ftr-high';
    if (score >= 0.5) return 'ftr-mid';
    return 'ftr-low';
  };

  const toolColumns = [
    { name: 'tool',         label: 'Tool',        sortable: true },
    { name: 'calls',        label: 'Calls',       sortable: true },
    { name: 'successPct',   label: 'Success',     sortable: true },
    { name: 'avgDuration',  label: 'Avg Duration',sortable: true },
  ];

  const toolRows = data.toolUsage.map((t: any) => ({
    tool: t.tool,
    calls: t.calls,
    successPct: `${Math.round(t.successRate * 100)}%`,
    avgDuration: t.avgDurationMs !== null
      ? (t.avgDurationMs >= 1000 ? `${(t.avgDurationMs / 1000).toFixed(1)}s` : `${t.avgDurationMs}ms`)
      : '—',
  }));
</script>

<a href="/repos/{data.repo.id}">← {data.repo.name}</a>
<h1>Analytics — {data.repo.name}</h1>
<p style="color:#64748b;font-size:0.875rem">Last 30 days</p>

<h2>Tool Usage</h2>
{#if data.toolUsage.length === 0}
  <p>No tool calls recorded yet. Start a session to see usage.</p>
{:else}
  <Table data={toolRows} columns={toolColumns} />
{/if}

<h2>Task Sessions ({data.sessions.length})</h2>
{#if data.sessions.length === 0}
  <p>No task sessions recorded yet. Pass <code>task_description</code> to <code>get_session_context</code> to start tracking.</p>
{:else}
  <div class="session-list">
    {#each data.sessions as session (session.id)}
      <div class="session-row">
        <div class="session-desc">{truncate(session.taskDescription)}</div>
        <div class="session-meta">
          {#if session.taskType}
            <span class="badge badge-type">{session.taskType}</span>
          {/if}
          <span class="badge badge-status-{session.status}">{session.status}</span>
          <span class="ftr {ftrColor(session.ftrScore)}">
            FTR {session.ftrScore !== null ? session.ftrScore.toFixed(3) : '—'}
          </span>
          <span class="ts">{fmt(session.createdAt)}</span>
        </div>
      </div>
    {/each}
  </div>
{/if}

<style>
  h2 { margin: 24px 0 12px; font-size: 1rem; font-weight: 600; color: #374151; }
  .session-list { display: flex; flex-direction: column; gap: 8px; }
  .session-row { display: flex; justify-content: space-between; align-items: center; padding: 10px 14px; border: 1px solid #e2e8f0; border-radius: 8px; background: #f8fafc; }
  .session-desc { font-size: 0.875rem; color: #1e293b; flex: 1; min-width: 0; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
  .session-meta { display: flex; align-items: center; gap: 10px; flex-shrink: 0; margin-left: 12px; }
  .badge { padding: 2px 8px; border-radius: 10px; font-size: 0.7rem; font-weight: 600; text-transform: uppercase; }
  .badge-type { background: #ede9fe; color: #5b21b6; }
  .badge-status-completed { background: #f1f5f9; color: #475569; }
  .badge-status-in_progress { background: #dbeafe; color: #1d4ed8; }
  .badge-status-abandoned { background: #fee2e2; color: #991b1b; }
  .ftr { font-size: 0.75rem; font-weight: 600; font-family: monospace; }
  .ftr-high { color: #166534; }
  .ftr-mid  { color: #92400e; }
  .ftr-low  { color: #991b1b; }
  .ftr-null { color: #94a3b8; }
  .ts { font-size: 0.75rem; color: #94a3b8; white-space: nowrap; }
</style>
```

- [ ] **Step 2: Add "View Analytics →" link to the repo page**

In `apps/dashboard/src/routes/repos/[id]/+page.svelte`, add after the existing sessions link:

```svelte
<p><a href="/repos/{data.repo.id}/sessions">View Sessions →</a></p>
<p><a href="/repos/{data.repo.id}/analytics">View Analytics →</a></p>
```

(The sessions link is at line 56. Add the analytics link directly after it.)

- [ ] **Step 3: Verify TypeScript compiles**

```bash
cd apps/dashboard && bunx tsc --noEmit
```

Expected: No errors

- [ ] **Step 4: Commit**

```bash
git add apps/dashboard/src/routes/repos/[id]/analytics/+page.svelte \
        apps/dashboard/src/routes/repos/[id]/+page.svelte
git commit -m "feat(dashboard): add analytics route with tool usage and FTR session list"
```

---

## Done When

- [ ] `packages/engine/src/analytics/ftr.spec.ts` — all tests pass
- [ ] `packages/engine/src/analytics/task-session.spec.ts` — all tests pass
- [ ] `packages/cli/src/commands/stats.spec.ts` — all tests pass
- [ ] `packages/engine` full suite (`cd packages/engine && bunx vitest run`) — no regressions
- [ ] `packages/cli` full suite (`cd packages/cli && bunx vitest run`) — no regressions
- [ ] `apps/dashboard` TypeScript compiles (`cd apps/dashboard && bunx tsc --noEmit`) — no errors
- [ ] `get_session_context` with `task_description` creates a `task_sessions` row
- [ ] Each tool call writes a `task_turns` row (verified via DB inspection or test)
- [ ] `checkpoint` writes FTR score to `task_sessions.ftr_score`
- [ ] `sensei stats` shows 7-day summary with sessions, avg FTR, and top tools
- [ ] Dashboard `/repos/[id]/analytics` shows tool usage table and session list with FTR badges
