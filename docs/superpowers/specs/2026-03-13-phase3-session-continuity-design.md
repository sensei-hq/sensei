---
id: session-continuity
type: spec
status: approved
date: 2026-03-13
---

# Session Continuity — Design Spec

## Goal

Add session tracking, snapshots, and project memory to sensei so that agents can recover from interruptions and carry knowledge forward across sessions. Four new MCP tools (`take_snapshot`, `checkpoint`, `record_memory`, `close_memory`) plus an enhanced `get_session_context` give agents a complete session lifecycle.

---

## Problem

Phase 2 gives agents smart context delivery for a single session. But sessions are interrupted — IDE crashes, token limits, accidental closes. When the next session starts, the agent has no memory of what it was doing, which decisions were made, or what conventions apply. Phase 3 adds persistent session state so agents orient in seconds rather than re-exploring from scratch.

---

## Architecture

```
Session lifecycle:
  get_session_context  →  creates session, detects crashes, surfaces recovery + memory
  take_snapshot        →  saves state at step boundaries (kind=manual)
  checkpoint           →  saves final state, marks session completed (kind=checkpoint)
  record_memory        →  stores decision / pattern / question in project memory
  close_memory         →  resolves an open question

Crash detection (on-demand):
  get_session_context checks for prior active sessions with last_heartbeat > 10 min ago
  → marks them crashed, fetches their latest snapshot for recovery context

Heartbeat:
  Every tool call updates last_heartbeat on the active session (silent best-effort)
```

Sessions and snapshots are write-mostly. Memory items are project-scoped and outlive any session.

**Note on `context_packs.session_id`:** The Phase 2 `context_packs` table has a `session_id text` column used as a free-form label (e.g., the agent passes its own identifier). This is independent of the Phase 3 `sessions.id uuid` primary key — they are different concepts. Phase 3 does not add a foreign key from `context_packs.session_id` to `sessions.id`. A future phase may unify them.

---

## Supabase Migration

`supabase/migrations/20260313000002_phase3_session_continuity.sql`

```sql
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
create index if not exists memory_items_repo_id_idx on sensei.memory_items(repo_id, type, status);

-- Grants (same pattern as Phase 1 and 2)
grant all on all tables in schema sensei to anon, authenticated, service_role;
grant all on all sequences in schema sensei to anon, authenticated, service_role;
grant execute on all functions in schema sensei to anon, authenticated, service_role;
```

DDL source files: `database/ddl/table/sensei/sessions.ddl`, `snapshots.ddl`, `memory_items.ddl`

---

## Engine Layer

### New Files

```
packages/engine/src/session/
  session-manager.ts   ← createSession, detectCrashedSessions, updateHeartbeat
  snapshot.ts          ← takeSnapshot, getLatestSnapshot
  memory.ts            ← recordMemory, closeMemory, getMemoryItems
```

### `session-manager.ts`

```typescript
interface SessionInfo {
  id: string;
  createdAt: string;
}

interface CrashedSession {
  id: string;
  createdAt: string;
  latestSnapshot: Snapshot | null;
}

async function createSession(db: SupabaseClient, repoId: string): Promise<SessionInfo>

async function detectCrashedSessions(
  db: SupabaseClient,
  repoId: string,
  idleThresholdMs?: number   // default 10 * 60 * 1000
): Promise<CrashedSession[]>

async function updateHeartbeat(db: SupabaseClient, sessionId: string): Promise<void>
// Silent best-effort — never throws
```

`detectCrashedSessions` finds sessions where `status = 'active'` AND `last_heartbeat < now() - idleThresholdMs`, updates them to `crashed`, and fetches their latest snapshot in one pass.

### `snapshot.ts`

```typescript
interface SnapshotOptions {
  kind: 'manual' | 'checkpoint';
  progressSummary: string;
  nextStepHint?: string;
  completedSteps?: string[];
  inFlightFiles?: string[];
  worktreeRefs?: Array<{ branch: string; path: string; status: string }>;
  diffStatSummary?: string;
}

interface Snapshot {
  id: string;
  kind: 'manual' | 'checkpoint';
  progressSummary: string;
  nextStepHint: string | null;
  completedSteps: string[];
  inFlightFiles: string[];
  worktreeRefs: Array<{ branch: string; path: string; status: string }>;
  diffStatSummary: string | null;
  createdAt: string;
}

async function takeSnapshot(
  db: SupabaseClient,
  sessionId: string,
  repoId: string,
  opts: SnapshotOptions
): Promise<Snapshot>
// Throws on DB error — agent must know if state wasn't saved

async function getLatestSnapshot(
  db: SupabaseClient,
  sessionId: string
): Promise<Snapshot | null>
```

### `memory.ts`

```typescript
interface MemoryItem {
  id: string;
  type: 'decision' | 'pattern' | 'question';
  title: string;
  content: string;
  status: 'open' | 'closed';
  resolution: string | null;
  createdAt: string;
}

async function recordMemory(
  db: SupabaseClient,
  repoId: string,
  sessionId: string,
  opts: { type: 'decision' | 'pattern' | 'question'; title: string; content: string }
): Promise<MemoryItem>
// Throws on DB error

async function closeMemory(
  db: SupabaseClient,
  itemId: string,
  resolution: string
): Promise<MemoryItem>
// Throws on DB error; throws if item not found or already closed

async function getMemoryItems(db: SupabaseClient, repoId: string): Promise<MemoryItem[]>
// Returns all items (open + closed) ordered by created_at desc
// Never throws — returns [] on error
```

---

## MCP Tools

### `get_session_context` (enhanced)

**Idempotency:** The MCP server holds the active `sessionId` in its in-memory closure. On the first call, a new session is created and the ID stored. On subsequent calls within the same server process, the existing session ID is reused — no new session row is created. This means calling `get_session_context` twice in one agent session is safe.

On each call:
1. Calls `detectCrashedSessions(db, repoId)` → marks any idle active sessions crashed, fetches their latest snapshots
2. If no session ID in memory: calls `createSession(db, repoId)` → new session row, status=`active`; stores ID in closure
3. Calls `getMemoryItems(db, repoId)` → all memory items
4. Returns enriched response:

```typescript
interface SessionContextResult {
  // existing fields
  repo_name: string;
  repo_path: string;
  symbol_count: number;
  file_count: number;
  last_indexed_at: string | null;
  stack: string[];
  // new fields
  session_id: string;
  interrupted: Array<{
    sessionId: string;
    crashedAt: string;          // last_heartbeat of the crashed session
    snapshot: Snapshot | null;  // most recent snapshot from that session
  }>;
  memory: {
    decisions: MemoryItem[];
    patterns: MemoryItem[];
    openQuestions: MemoryItem[];  // status=open questions only
  };
  message: string;              // updated to mention interrupted sessions if any
}
```

### `take_snapshot`

```typescript
// Input
{
  progress_summary: string;           // required — what you're doing right now
  next_step_hint?: string;            // what to do next if interrupted
  in_flight_files?: string[];         // files currently being modified
  completed_steps?: string[];         // steps finished so far this task
  worktree_refs?: Array<{ branch: string; path: string; status: string }>;
  diff_stat_summary?: string;         // e.g. "8 files changed, +142 -31"
}
// Output: { id, kind: "manual", progressSummary, createdAt }
```

Updates `last_heartbeat`. Throws on DB error.

### `checkpoint`

```typescript
// Input
{
  task_summary: string;       // required — what was accomplished
  completed_steps?: string[]; // final list of completed steps
}
// Output: { id, kind: "checkpoint", progressSummary, createdAt }
```

Writes a `checkpoint` snapshot, then marks the session `status = 'completed'`. Future `get_session_context` calls will not surface this session as interrupted.

**After checkpoint:** The session remains `completed` in the DB. Subsequent `take_snapshot` or `record_memory` calls in the same server process still succeed — they write to the completed session (useful if the agent continues working after a task boundary). A new active session is only created on the next `get_session_context` call in a new server process.

### `record_memory`

```typescript
// Input
{
  type: "decision" | "pattern" | "question";
  title: string;    // short label, e.g. "Use optimistic locking for invoice updates"
  content: string;  // full description
}
// Output: { id, type, title, status: "open", createdAt }
```

### `close_memory`

```typescript
// Input
{
  id: string;         // memory item ID returned by record_memory
  resolution: string; // how the question was resolved
}
// Output: { id, type, title, status: "closed", resolution, closedAt }
```

Throws if item not found or already closed.

### Heartbeat Middleware

In `mcp-server.ts`, a shared `withHeartbeat` wrapper calls `updateHeartbeat(db, sessionId)` after each tool handler completes. Failures are silently swallowed — a missed heartbeat only widens the crash detection window.

```typescript
// Pattern in mcp-server.ts
const beat = () => updateHeartbeat(db, sessionId).catch(() => {});

server.tool("take_snapshot", ..., async (params) => {
  const result = await takeSnapshotTool(...);
  beat();
  return result;
});
```

---

## New Files

```
supabase/migrations/
  20260313000002_phase3_session_continuity.sql

packages/engine/src/session/
  session-manager.ts
  session-manager.spec.ts
  snapshot.ts
  snapshot.spec.ts
  memory.ts
  memory.spec.ts

packages/engine/src/index.ts          ← add session exports

packages/server/src/tools/
  take-snapshot.ts
  checkpoint.ts
  record-memory.ts
  close-memory.ts

packages/server/src/tools/get-session-context.ts   ← enhanced
packages/server/src/mcp-server.ts                  ← session closure + heartbeat + 4 new tools

apps/dashboard/src/routes/repos/[id]/sessions/
  +page.server.ts
  +page.svelte

apps/dashboard/src/routes/repos/[id]/+page.svelte  ← add "View Sessions →" link
```

---

## Dashboard — Sessions Inspector

Route: `/repos/[id]/sessions`

Server load: queries `sensei.sessions` for the repo ordered by `created_at desc`, with snapshot count and memory item count per session.

UI (Rokkit components, matching existing dashboard patterns):
- `List` of sessions — status badge (`active` / `completed` / `crashed`), created timestamp, last heartbeat
- Expanding a session shows:
  - Snapshot timeline: `Table` of snapshots (kind, progressSummary, createdAt)
  - Memory items created in this session: `Table` of items (type, title, status)

---

## Error Handling

| Operation | On error |
|-----------|----------|
| `detectCrashedSessions` | Returns `[]`, orientation continues without recovery context |
| `createSession` | Throws — orientation fails clearly |
| `getMemoryItems` | Returns `[]`, orientation continues without memory |
| `take_snapshot` | Throws — agent is informed state was not saved |
| `checkpoint` | Throws — agent is informed task was not checkpointed |
| `record_memory` | Throws — agent is informed memory was not stored |
| `close_memory` | Throws — including if item not found or already closed |
| `updateHeartbeat` | Silent — missed heartbeat widens crash window, never blocks |

---

## Done When

- `get_session_context` creates a session and returns `session_id`, `interrupted`, and `memory` fields
- `take_snapshot` writes a manual snapshot and returns its ID
- `checkpoint` writes a checkpoint snapshot and marks the session `completed`
- `record_memory` creates a memory item (decision / pattern / question)
- `close_memory` resolves an open question
- Hard-killing the MCP server and starting a new session surfaces the crashed session's last snapshot in `get_session_context`
- Dashboard `/repos/[id]/sessions` lists sessions with status badges and expandable snapshot timelines
