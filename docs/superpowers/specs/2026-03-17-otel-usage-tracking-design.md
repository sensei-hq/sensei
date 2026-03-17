# OTel Usage Tracking Design

> **Status:** approved
> **Date:** 2026-03-17

## Goal

Capture Claude Code's built-in OpenTelemetry events (`claude_code.api_request`) to track token counts and cost per session, and enable benchmark comparisons between runs with and without sensei.

## Problem

The existing `task_sessions` and `task_turns` tables track tool calls and FTR scores but contain no token or cost data. Claude Code fires `claude_code.api_request` OTel events per API call with input_tokens, output_tokens, cache tokens, cost_usd, model, duration_ms, and prompt.id. This data is currently discarded.

## Solution

Add an OTLP HTTP endpoint to the existing sensei MCP server process. Claude Code sends events there. The endpoint stores rows in a new `api_requests` table, correlated with `task_sessions` by time window. A `benchmark_runs` table pairs with/without-sensei runs for direct cost comparison.

A dry-run mode (`SENSEI_OTEL_DRY_RUN=true`) logs parsed events to stdout for verification before any DB writes are enabled.

---

## Architecture

### OTLP Endpoint (`packages/server/src/otlp-endpoint.ts`)

- HTTP listener on port 4318 (OTLP HTTP standard)
- Accepts `POST /v1/metrics` (OTLP JSON format)
- Parses `claude_code.api_request` metric events, extracts:
  - `input_tokens`, `output_tokens`, `cache_read_tokens`, `cache_creation_tokens`
  - `cost_usd`, `duration_ms`, `model`, `prompt.id`
- **Dry-run mode** (`SENSEI_OTEL_DRY_RUN=true`): logs parsed event to stdout, no DB write
- **Write mode**: inserts row into `sensei.api_requests`, correlates to `task_session_id` via time window lookup

### Session Correlation

On insert, the endpoint queries `task_sessions` for the row with:
- `repo_id` matching the current server's repo
- `status = 'in_progress'` at `recorded_at` (i.e., `created_at <= recorded_at AND (completed_at IS NULL OR completed_at >= recorded_at)`)

The matched `task_session_id` is attached to the `api_requests` row. If no session is active, `task_session_id` is left null (still stored for overall usage totals).

### MCP Server Integration (`packages/server/src/serve.ts`)

The OTLP endpoint starts alongside the existing MCP server. Same process, separate HTTP port. Startup logs the port and mode (dry-run or live).

---

## Database

### `sensei.api_requests`

One row per Claude Code API call.

```sql
create table if not exists api_requests (
  id                    uuid          primary key default gen_random_uuid()
, repo_id               uuid          not null references sensei.repos(id) on delete cascade
, task_session_id       uuid          references sensei.task_sessions(id) on delete set null
, benchmark_run_id      uuid          references sensei.benchmark_runs(id) on delete set null
, prompt_id             text          not null
, input_tokens          int           not null
, output_tokens         int           not null
, cache_read_tokens     int           not null default 0
, cache_creation_tokens int           not null default 0
, cost_usd              numeric(10,6) not null
, duration_ms           int
, model                 text
, recorded_at           timestamptz   not null default now()
);
```

Indexes: `(repo_id, recorded_at desc)`, `(task_session_id)`, `(benchmark_run_id)`, `(prompt_id)`.

### `sensei.benchmark_runs`

One row per benchmark run (paired: same task, same branch, sensei_enabled true/false).

```sql
create table if not exists benchmark_runs (
  id                          uuid          primary key default gen_random_uuid()
, repo_id                     uuid          not null references sensei.repos(id) on delete cascade
, task_description            text          not null
, branch                      text          not null
, sensei_enabled              boolean       not null
, started_at                  timestamptz   not null default now()
, ended_at                    timestamptz
, total_cost_usd              numeric(10,6)
, total_input_tokens          int
, total_output_tokens         int
, total_cache_read_tokens     int
, total_cache_creation_tokens int
, worktree_path               text
);
```

Index: `(repo_id, task_description, branch)` — for pairing runs.

### DDL Files

Both tables get canonical DDL files in `database/ddl/table/sensei/`:
- `database/ddl/table/sensei/api_requests.ddl`
- `database/ddl/table/sensei/benchmark_runs.ddl`

Follow the exact format of existing DDL files (set search_path, create table, indexes, comment on table).

---

## Benchmark Pair Protocol (running-benchmarks skill)

Updated procedure:

1. **Create benchmark worktree** on branch `benchmark/<task>-<timestamp>`
2. **No-sensei run**:
   - Strip sensei from worktree's `.mcp.json` (keep OTel env vars)
   - Insert `benchmark_runs` row (`sensei_enabled: false`)
   - Launch Claude Code with task prompt + OTel enabled
   - On completion: aggregate `api_requests` rows by time window into `benchmark_runs` totals
3. **Sensei run**:
   - Restore full `.mcp.json` in worktree
   - Insert `benchmark_runs` row (`sensei_enabled: true`)
   - Same task prompt + OTel
   - Aggregate totals
4. **Delete worktree**
5. Dashboard shows the pair side-by-side

**Environment for all benchmark runs:**
```
CLAUDE_CODE_ENABLE_TELEMETRY=1
OTEL_METRICS_EXPORTER=otlp
OTEL_EXPORTER_OTLP_ENDPOINT=http://localhost:4318
SENSEI_OTEL_DRY_RUN=false
```

---

## Dashboard

`/repos/[id]/analytics` gets a new "Cost" section:

**Session cost breakdown** (below existing tool usage table):
- Table: task description, date, input tokens, output tokens, cache tokens, total cost
- Sourced from `api_requests` joined to `task_sessions`

**Benchmark comparison** (new subsection, only shown when benchmark pairs exist):
- Table: task description, branch, with-sensei cost, without-sensei cost, savings ($), savings (%)
- Sourced from `benchmark_runs` paired by `(task_description, branch)`

---

## Verification Procedure

Before writing to the DB, verify events flow correctly:

1. Set env vars:
   ```
   CLAUDE_CODE_ENABLE_TELEMETRY=1
   OTEL_METRICS_EXPORTER=otlp
   OTEL_EXPORTER_OTLP_ENDPOINT=http://localhost:4318
   SENSEI_OTEL_DRY_RUN=true
   ```
2. Start sensei MCP server
3. Run a Claude Code task
4. Confirm the console logs show parsed `claude_code.api_request` events with correct field values
5. Remove `SENSEI_OTEL_DRY_RUN` to enable DB writes

---

## Files Created / Modified

| File | Action |
|------|--------|
| `packages/server/src/otlp-endpoint.ts` | Create |
| `packages/server/src/serve.ts` | Modify — start OTLP endpoint |
| `supabase/migrations/20260317000001_otel_api_requests.sql` | Create |
| `supabase/migrations/20260317000002_benchmark_runs.sql` | Create |
| `database/ddl/table/sensei/api_requests.ddl` | Create |
| `database/ddl/table/sensei/benchmark_runs.ddl` | Create |
| `apps/dashboard/src/routes/repos/[id]/analytics/+page.server.ts` | Modify |
| `apps/dashboard/src/routes/repos/[id]/analytics/+page.svelte` | Modify |
| `skills/running-benchmarks/SKILL.md` | Modify |
