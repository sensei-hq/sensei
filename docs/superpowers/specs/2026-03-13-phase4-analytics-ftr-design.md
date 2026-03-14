---
id: analytics-ftr
type: spec
status: approved
date: 2026-03-13
---

# Analytics & FTR — Design Spec

## Goal

Add task session tracking and First-Time-Right (FTR) scoring to sensei so developers can measure how effectively agents complete tasks, visible in a dashboard and CLI.

---

## Problem

The collector daemon already writes every tool call to `sensei.events`. But there is no task-level grouping, no completion signal, and no quality score. Phase 4 adds `task_sessions` and `task_turns` tables, heuristic FTR computation at checkpoint time, a `sensei stats` CLI command, and an analytics dashboard route.

---

## Architecture

```
Agent flow:
  get_session_context(task_description?) → creates task_session row
  [tool calls]                           → task_turns rows via heartbeat wrapper
  checkpoint(task_summary)               → completes task_session, computes FTR

Analytics:
  sensei stats                           → 7-day summary from task_sessions + task_turns
  /repos/[id]/analytics                  → tool usage table + session list with FTR
```

Task sessions are linked to Phase 3 sessions via `task_sessions.session_id` (FK, nullable on session delete). `task_turns` are written by the existing `beat()` wrapper in `mcp-server.ts` — one row per post-phase tool call.

---

## Supabase Migration

`supabase/migrations/20260313000003_phase4_analytics_ftr.sql`

```sql
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

DDL source files: `database/ddl/table/sensei/task_sessions.ddl`, `task_turns.ddl`

---

## Engine Layer

### New Files

```
packages/engine/src/analytics/
  task-session.ts   ← createTaskSession, completeTaskSession, getTaskSessions
  task-session.spec.ts
  ftr.ts            ← computeFtr (pure), computeAndStoreFtr
  ftr.spec.ts

packages/engine/src/index.ts  ← add analytics exports
```

### `ftr.ts`

```typescript
export interface FtrSignals {
  snapshotCount: number;
  toolErrorRate: number;    // 0.0–1.0
  completedCleanly: boolean;
  hasDescription: boolean;
}

export interface FtrResult {
  score: number;            // 0.000–1.000
  signals: FtrSignals;
}

export function computeFtr(signals: FtrSignals): number
// Pure function — no DB access. Penalties:
//   snapshot count > 1: -0.05 per extra, capped at -0.30
//   toolErrorRate >= 0.20: -0.20; >= 0.10: -0.10
//   completedCleanly = false: -0.30
//   hasDescription = false: cap final score at 0.70
// Result clamped to [0.0, 1.0]

export async function computeAndStoreFtr(
  db: SupabaseClient,
  taskSessionId: string,
  sessionId: string
): Promise<FtrResult>
// Fetches snapshot count (all kinds: manual + checkpoint) from sensei.snapshots where session_id = sessionId
// Fetches task_turns error rate from sensei.task_turns where task_session_id = taskSessionId
// Fetches session status from sensei.sessions; completedCleanly = status === 'completed'
//   (checkpointTool sets status='completed' before this is called, so this will be true on the clean path)
// Calls computeFtr, stores ftr_score + ftr_signals on task_sessions row
// Throws on DB error
```

### `task-session.ts`

```typescript
export interface TaskSessionInfo {
  id: string;
  taskType: string | null;
  createdAt: string;
}

export async function createTaskSession(
  db: SupabaseClient,
  sessionId: string,
  repoId: string,
  taskDescription?: string
): Promise<TaskSessionInfo>
// Auto-detects task_type from taskDescription keywords:
//   fix|bug|patch → 'fix'
//   feat|add|implement|build → 'feat'
//   refactor|clean|extract → 'refactor'
//   docs|document → 'docs'
//   test|spec → 'test'
//   chore|bump|update|upgrade → 'chore'
//   else → 'unknown'
// Throws on DB error

export async function completeTaskSession(
  db: SupabaseClient,
  taskSessionId: string,
  sessionId: string
): Promise<FtrResult>
// Calls computeAndStoreFtr, then sets status='completed' + completed_at=now()
// Throws on DB error

export async function recordTaskTurn(
  db: SupabaseClient,
  taskSessionId: string,
  repoId: string,
  tool: string,
  success: boolean | null,
  durationMs?: number | null
): Promise<void>
// Inserts a task_turns row (includes repo_id for direct indexed queries). Silent best-effort — never throws.

export async function getTaskSessions(
  db: SupabaseClient,
  repoId: string,
  limitDays?: number  // default 30
): Promise<TaskSession[]>
// Returns task_sessions for repo ordered by created_at desc, within limitDays
// Never throws — returns [] on error

export interface TaskSession {
  id: string;
  sessionId: string | null;
  repoId: string;
  taskDescription: string | null;
  taskType: 'feat' | 'fix' | 'refactor' | 'docs' | 'test' | 'chore' | 'unknown' | null;
  status: 'in_progress' | 'completed' | 'abandoned';
  ftrScore: number | null;
  ftrSignals: FtrSignals | null;
  createdAt: string;
  completedAt: string | null;
}
```

---

## MCP Changes

### `get_session_context` (enhanced)

Adds optional `task_description` string input parameter.

On first call (session created):
1. Creates session (existing Phase 3 behaviour)
2. Calls `createTaskSession(db, sessionId, repoId, taskDescription)` → stores `taskSessionId` in closure

Returns existing `SessionContextResult` unchanged — `taskSessionId` is internal only.

**Idempotency:** subsequent calls in same server process reuse existing `taskSessionId`.

### `checkpoint` (enhanced in `mcp-server.ts`)

`checkpointTool` in `checkpoint.ts` is unchanged — it still sets `sessions.status = 'completed'` as before.

In `mcp-server.ts`, the checkpoint handler calls `completeTaskSession` directly after `checkpointTool` returns, using the closure's `taskSessionId`:

```typescript
server.tool("checkpoint", ..., async (params) => {
  const client = await getClient();
  const result = await checkpointTool(client, sessionId!, repoId, params);
  // completeTaskSession runs after checkpointTool so sessions.status is already 'completed'
  if (taskSessionId) {
    await completeTaskSession(client, taskSessionId, sessionId!).catch(() => {});
  }
  beat(client, "checkpoint", true);
  return result;
});
```

If `taskSessionId` is null (agent never provided a task_description), `completeTaskSession` is skipped — no FTR row is written for that session.

### Heartbeat wrapper (enhanced)

`beat` signature changes to accept `toolName` and `success` alongside `client`. The `client` parameter is retained (it is resolved per-call in each handler and passed in, matching the existing pattern):

```typescript
// In mcp-server.ts
let taskSessionId: string | null = null;

const beat = (client: SupabaseClient, toolName: string, success: boolean) => {
  if (sessionId) updateHeartbeat(client, sessionId).catch(() => {});
  if (taskSessionId) recordTaskTurn(client, taskSessionId, opts.repoId, toolName, success).catch(() => {});
};
```

Every existing tool handler changes from `beat(client)` to `beat(client, "<tool_name>", true)`. Handlers that can fail pass `false` for success in their error path.

---

## CLI — `sensei stats`

`packages/cli/src/commands/stats.ts` — **replaces** the existing implementation that reads from `@sensei/collector`. The new implementation reads from Supabase `task_sessions` + `task_turns`. The existing `stats.spec.ts` test file is updated to match the new data source and output shape.

```
sensei stats (last 7 days)

Sessions        12   (8 completed, 3 abandoned, 1 in_progress)
Avg FTR          0.74
Top tools        Bash (142), Read (98), Edit (71), Write (34)
Errors           9 tool failures across 4 sessions
```

`--json` flag outputs the same data as valid JSON.

Reads: `task_sessions` joined to `task_turns` for the repo, filtered to `created_at >= now() - 7 days`. `task_turns` queries use the `repo_id` index directly (no join required).

---

## Dashboard — Analytics Route

New route: `/repos/[id]/analytics`

**Server load** (`+page.server.ts`): returns two shapes:
- `sessions: TaskSession[]` — all `task_sessions` for repo, last 30 days, ordered by `created_at desc`
- `toolUsage: Array<{ tool: string; calls: number; successRate: number; avgDurationMs: number | null }>` — aggregated from `task_turns` where `repo_id = repoId` and `created_at >= now() - 30 days`, grouped client-side by `tool`, top 10 by call count

Both queries use the `repo_id` indexes. No RPCs needed.

**UI** (`+page.svelte`, Rokkit components, matching existing patterns):

- **Tool usage table** — top 10 tools by call count, columns: tool, calls, success rate, avg duration
- **Session list** — each `task_session` as a row: task description (truncated to 60 chars), task type badge, status badge, FTR score (color-coded: ≥0.8 green / 0.5–0.79 yellow / <0.5 red / null grey), created_at timestamp

**Link added to** `apps/dashboard/src/routes/repos/[id]/+page.svelte`: `<a href="/repos/{data.repo.id}/analytics">View Analytics →</a>`

---

## Error Handling

| Operation | On error |
|-----------|----------|
| `createTaskSession` | Throws — orientation fails clearly |
| `completeTaskSession` | Throws — agent informed FTR not computed |
| `recordTaskTurn` | Silent — missed turns widen error rate estimate, never block |
| `getTaskSessions` | Returns [] — dashboard shows empty state |
| `computeAndStoreFtr` | Throws (surfaced by completeTaskSession) |

---

## Test Scope

**`ftr.spec.ts`** must cover at minimum:
- Score = 1.0 with perfect signals (snapshotCount=1, errorRate=0, completedCleanly=true, hasDescription=true)
- Each penalty individually (snapshot penalty, error rate tiers, crashed penalty)
- Score clamped at 0.0 when all penalties stack
- `hasDescription=false` caps score at 0.70 regardless of other signals

**`task-session.spec.ts`** must cover:
- `createTaskSession` with description → correct task_type auto-detection for each keyword group
- `createTaskSession` without description → task_type='unknown'
- `completeTaskSession` → calls `computeAndStoreFtr`, sets status='completed' + completed_at
- `recordTaskTurn` swallows DB errors (never throws)

---

## New Files

```
supabase/migrations/
  20260313000003_phase4_analytics_ftr.sql

database/ddl/table/sensei/
  task_sessions.ddl
  task_turns.ddl

packages/engine/src/analytics/
  task-session.ts
  task-session.spec.ts
  ftr.ts
  ftr.spec.ts

packages/engine/src/index.ts          ← add analytics exports

packages/server/src/tools/
  get-session-context.ts              ← add task_description param + createTaskSession

packages/server/src/mcp-server.ts     ← taskSessionId closure, beat() signature change, completeTaskSession call in checkpoint handler

packages/cli/src/commands/stats.ts    ← read from Supabase, add --json

apps/dashboard/src/routes/repos/[id]/analytics/
  +page.server.ts
  +page.svelte

apps/dashboard/src/routes/repos/[id]/+page.svelte  ← add "View Analytics →" link
```

---

## Done When

- `get_session_context` with `task_description` creates a task_session row
- Each tool call writes a `task_turns` row (silent best-effort)
- `checkpoint` computes FTR score and stores it on the task_session
- `sensei stats` shows 7-day summary with avg FTR and top tools
- Dashboard `/repos/[id]/analytics` lists sessions with FTR scores and tool usage table
