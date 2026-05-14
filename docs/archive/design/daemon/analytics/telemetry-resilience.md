---
id: telemetry-resilience
type: design
status: active
---

# Telemetry Resilience

> **SUPERSEDED:** This design predates the PostgreSQL migration. The outbox pattern described here was not implemented. Events are now stored directly in `activity.events` (PostgreSQL). See ADR-005.

> How sensei handles Supabase unavailability — local buffering, durable queue, and retry for both tool events and OTLP API-cost data.

---

## Problem

Sensei collects two streams of telemetry:

| Stream | Source | Destination |
|---|---|---|
| Tool events (pre/post) | Hook scripts → daemon `/event` | `sensei.tool_events` |
| OTLP API-cost events | Claude Code → daemon `/v1/logs` | `sensei.api_requests` |

Both streams write to Supabase. If Supabase is unavailable (network down, local instance stopped, transient error), events are currently handled inconsistently:

- **Tool events** — daemon has a JSONL fallback file; drained on next startup. But only for the startup drain, not for failures that happen after startup.
- **OTLP events** — no fallback at all. Events are silently dropped.

---

## Current Fallback (Tool Events)

The daemon has a partial solution:

```
Hook fires → POST /event → Supabase write succeeds → done
                         ↓ fails
                         Append to events.jsonl
                         Next daemon start → drainJsonl() → retry
```

Gaps:
1. JSONL is only drained at startup — events buffered during a long-running daemon session aren't retried until restart
2. OTLP path has no fallback at all
3. No visibility into what's buffered

---

## Proposed Design

### Transactional outbox in SQLite

The collector daemon already maintains `~/.sensei/<uuid>/analytics.db`. Add an `outbox` table as a durable write buffer:

```sql
CREATE TABLE outbox (
  id          INTEGER PRIMARY KEY AUTOINCREMENT,
  stream      TEXT    NOT NULL,  -- "tool_event" | "otlp"
  payload     TEXT    NOT NULL,  -- JSON
  created_at  INTEGER NOT NULL,  -- unix ms
  attempts    INTEGER NOT NULL DEFAULT 0,
  last_error  TEXT
);
```

**Write path (hot):**

```
Incoming event → try Supabase insert
                 ✓ success → done
                 ✗ error   → INSERT into outbox
```

**Retry path (background):**

```
Every 30s: SELECT * FROM outbox ORDER BY created_at LIMIT 50
           For each row: retry Supabase insert
           ✓ success → DELETE FROM outbox
           ✗ error   → UPDATE attempts++, last_error
```

**Eviction:**

```
Prune outbox rows where attempts > 20 OR created_at < now - 7 days
```

This gives:
- No events lost during Supabase downtime (up to 7 days / 20 retries)
- Automatic recovery when Supabase comes back
- No daemon restart required

### Retry backoff

| Attempt | Wait before next retry |
|---|---|
| 1–3 | 30s (normal interval) |
| 4–6 | 5 min |
| 7–10 | 30 min |
| 11+ | 6 hours |

The retry loop runs at the base interval; the per-row backoff is enforced by checking `last_attempted_at + backoff > now`.

### JSONL migration

The existing JSONL fallback (written by hook scripts when the daemon is unreachable) is retained as a **pre-daemon** fallback. When the daemon starts, it drains the JSONL into the outbox (not directly to Supabase). This makes JSONL→Supabase delivery consistent with the outbox retry path.

```
Hook script → POST /event → (daemon unreachable) → append to events.jsonl
Daemon starts → drainJsonl() → INSERT into outbox → retry loop picks up
```

---

## Implementation Plan

### Phase 1 — SQLite outbox for tool events

1. `packages/collector/src/schema.ts` — add `outbox` table to SQLite schema
2. `packages/collector/src/daemon.ts` — wrap `writeEventToSupabase` in try/catch; on error, insert into outbox
3. `packages/collector/src/daemon.ts` — start background retry loop in `startDaemon`
4. `packages/collector/src/drain.ts` — update `drainJsonl` to insert into outbox instead of calling Supabase directly

### Phase 2 — Outbox for OTLP events

5. `packages/collector/src/daemon.ts` — wrap OTLP Supabase inserts in try/catch with outbox fallback
6. Add outbox retry to handle both `tool_event` and `otlp` stream types

### Phase 3 — Observability

7. `GET /health` — include outbox queue depth in response: `{ ok: true, outbox: { queued: 12, oldest_ms: 300000 } }`
8. `sensei doctor` — warn if outbox has entries older than 1 hour

---

## What This Does NOT Cover

- **Daemon crash** — if the daemon process dies, hook events fall to JSONL (existing behavior). The JSONL is drained when the daemon restarts.
- **SQLite corruption** — unlikely but possible. The JSONL file is the fallback for this case; both buffers existing simultaneously is acceptable redundancy.
- **Duplicate delivery** — if a Supabase write partially succeeds, the retry may insert duplicates. Mitigation: add a unique index on `outbox.id` cross-referenced with a `source_id` field; or accept rare duplicates (analytics data, not transactional).
