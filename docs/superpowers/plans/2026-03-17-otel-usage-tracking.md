# OTel Usage Tracking Implementation Plan

> **For agentic workers:** REQUIRED: Use superpowers:subagent-driven-development (if subagents available) or superpowers:executing-plans to implement this plan. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add an OTLP HTTP endpoint to the sensei MCP server that captures Claude Code's `claude_code.api_request` telemetry events, stores token/cost data per session in Supabase, and enables benchmark cost comparisons between with/without-sensei runs.

**Architecture:** A new `otlp-endpoint.ts` module creates a Bun HTTP server on port 4318 that accepts OTLP JSON metrics and logs, parses `claude_code.api_request` events, and either logs them (dry-run) or writes to a new `api_requests` Supabase table. A `benchmark_runs` table pairs identical tasks run with and without sensei for cost comparison. Both tables are defined in `database/ddl/table/sensei/` and deployed via Supabase migrations.

**Tech Stack:** TypeScript, Bun HTTP, OTLP JSON, Supabase, SvelteKit, Vitest

---

## Chunk 1: Database Layer

### Task 1: DDL files for api_requests and benchmark_runs

**Files:**
- Create: `database/ddl/table/sensei/api_requests.ddl`
- Create: `database/ddl/table/sensei/benchmark_runs.ddl`

No tests needed — these are schema definition files. Follow the exact format of `database/ddl/table/sensei/task_turns.ddl` (set search_path, create table if not exists, indexes, comment on table).

- [ ] **Step 1: Create `database/ddl/table/sensei/api_requests.ddl`**

```sql
set search_path to sensei, extensions;

create table if not exists api_requests (
  id                      uuid          primary key default gen_random_uuid()
, repo_id                 uuid          not null references sensei.repos(id) on delete cascade
, task_session_id         uuid          references sensei.task_sessions(id) on delete set null
, benchmark_run_id        uuid          references sensei.benchmark_runs(id) on delete set null
, prompt_id               text          not null
, input_tokens            int           not null
, output_tokens           int           not null
, cache_read_tokens       int           not null default 0
, cache_creation_tokens   int           not null default 0
, cost_usd                numeric(10,6) not null
, duration_ms             int
, model                   text
, recorded_at             timestamptz   not null default now()
);

create index if not exists api_requests_repo_recorded_idx
  on api_requests(repo_id, recorded_at desc);
create index if not exists api_requests_task_session_idx
  on api_requests(task_session_id)
  where task_session_id is not null;
create index if not exists api_requests_benchmark_run_idx
  on api_requests(benchmark_run_id)
  where benchmark_run_id is not null;
create index if not exists api_requests_prompt_id_idx
  on api_requests(prompt_id);

comment on table api_requests is
'One row per Claude Code API call captured via OTLP telemetry.
- prompt_id: Claude Code prompt/task identifier from OTel attribute
- task_session_id: linked sensei task session (correlated by time window, nullable)
- benchmark_run_id: linked benchmark run if this call occurred during a benchmark
- cost_usd: total cost for this API call as reported by Claude Code
- cache_*_tokens: separate cache read vs cache creation token counts for cost breakdown';
```

- [ ] **Step 2: Create `database/ddl/table/sensei/benchmark_runs.ddl`**

```sql
set search_path to sensei, extensions;

create table if not exists benchmark_runs (
  id                        uuid          primary key default gen_random_uuid()
, repo_id                   uuid          not null references sensei.repos(id) on delete cascade
, task_description          text          not null
, branch                    text          not null
, sensei_enabled            boolean       not null
, started_at                timestamptz   not null default now()
, ended_at                  timestamptz
, total_cost_usd            numeric(10,6)
, total_input_tokens        int
, total_output_tokens       int
, total_cache_read_tokens   int
, total_cache_creation_tokens int
, worktree_path             text
);

create index if not exists benchmark_runs_repo_idx
  on benchmark_runs(repo_id, task_description, branch);

comment on table benchmark_runs is
'One row per benchmark execution (paired: same task + branch, sensei_enabled true/false).
- sensei_enabled: false = no sensei MCP tools available; true = normal sensei config
- started_at / ended_at: time window used to correlate api_requests rows
- total_* columns: aggregated from api_requests rows after run completes
- worktree_path: path of the temporary benchmark worktree (deleted after run)
Pairs are matched by (repo_id, task_description, branch) with different sensei_enabled values.';
```

- [ ] **Step 3: Verify files exist**

```bash
head -1 database/ddl/table/sensei/api_requests.ddl
head -1 database/ddl/table/sensei/benchmark_runs.ddl
```
Expected: first line of each is `set search_path to sensei, extensions;`

- [ ] **Step 4: Commit**

```bash
git add database/ddl/table/sensei/api_requests.ddl database/ddl/table/sensei/benchmark_runs.ddl
git commit -m "feat(db): add api_requests and benchmark_runs DDL definitions"
```

---

### Task 2: Supabase migrations

**Files:**
- Create: `supabase/migrations/20260317000001_benchmark_runs.sql`
- Create: `supabase/migrations/20260317000002_otel_api_requests.sql`

`api_requests` references `benchmark_runs` via FK, so `benchmark_runs` must be created first. Use timestamps so `benchmark_runs` sorts earlier:
- `20260317000001_benchmark_runs.sql` — runs first
- `20260317000002_otel_api_requests.sql` — references benchmark_runs

- [ ] **Step 1: Create `supabase/migrations/20260317000001_benchmark_runs.sql`**

```sql
-- benchmark_runs: one row per benchmark execution (with/without sensei)
create table if not exists sensei.benchmark_runs (
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

create index if not exists benchmark_runs_repo_idx
  on sensei.benchmark_runs(repo_id, task_description, branch);
```

- [ ] **Step 2: Create `supabase/migrations/20260317000002_otel_api_requests.sql`**

```sql
-- api_requests: one row per Claude Code API call captured via OTLP
create table if not exists sensei.api_requests (
  id                      uuid          primary key default gen_random_uuid()
, repo_id                 uuid          not null references sensei.repos(id) on delete cascade
, task_session_id         uuid          references sensei.task_sessions(id) on delete set null
, benchmark_run_id        uuid          references sensei.benchmark_runs(id) on delete set null
, prompt_id               text          not null
, input_tokens            int           not null
, output_tokens           int           not null
, cache_read_tokens       int           not null default 0
, cache_creation_tokens   int           not null default 0
, cost_usd                numeric(10,6) not null
, duration_ms             int
, model                   text
, recorded_at             timestamptz   not null default now()
);

create index if not exists api_requests_repo_recorded_idx
  on sensei.api_requests(repo_id, recorded_at desc);
create index if not exists api_requests_task_session_idx
  on sensei.api_requests(task_session_id)
  where task_session_id is not null;
create index if not exists api_requests_benchmark_run_idx
  on sensei.api_requests(benchmark_run_id)
  where benchmark_run_id is not null;
create index if not exists api_requests_prompt_id_idx
  on sensei.api_requests(prompt_id);
```

- [ ] **Step 3: Verify migration files**

```bash
ls supabase/migrations/ | grep 202603170000
```
Expected: `20260317000000_pattern_usages.sql`, `20260317000001_benchmark_runs.sql`, `20260317000002_otel_api_requests.sql`

- [ ] **Step 4: Commit**

```bash
git add supabase/migrations/20260317000001_benchmark_runs.sql supabase/migrations/20260317000002_otel_api_requests.sql
git commit -m "feat(db): add benchmark_runs and api_requests Supabase migrations"
```

---

## Chunk 2: OTLP Endpoint

### Task 3: `otlp-endpoint.ts` with tests

**Files:**
- Create: `packages/server/src/otlp-endpoint.ts`
- Create: `packages/server/src/otlp-endpoint.spec.ts`

OTLP JSON format: Claude Code sends `POST /v1/metrics` or `POST /v1/logs`. Parse `claude_code.api_request` from either. Attributes are in OTLP key-value format: `[{ key: "input_tokens", value: { intValue: "1234" } }, ...]`.

- [ ] **Step 1: Write the failing tests**

```typescript
// packages/server/src/otlp-endpoint.spec.ts
import { describe, it, expect, vi, beforeEach, afterEach } from "vitest";
import { createOtlpEndpoint } from "./otlp-endpoint.js";

const METRICS_BODY = {
  resourceMetrics: [{
    scopeMetrics: [{
      metrics: [{
        name: "claude_code.api_request",
        gauge: {
          dataPoints: [{
            attributes: [
              { key: "prompt.id", value: { stringValue: "prompt-abc" } },
              { key: "input_tokens", value: { intValue: "1000" } },
              { key: "output_tokens", value: { intValue: "500" } },
              { key: "cache_read_tokens", value: { intValue: "200" } },
              { key: "cache_creation_tokens", value: { intValue: "50" } },
              { key: "cost_usd", value: { doubleValue: 0.015 } },
              { key: "duration_ms", value: { intValue: "2500" } },
              { key: "model", value: { stringValue: "claude-sonnet-4-6" } },
            ],
          }],
        },
      }],
    }],
  }],
};

const LOGS_BODY = {
  resourceLogs: [{
    scopeLogs: [{
      logRecords: [{
        attributes: [
          { key: "event.name", value: { stringValue: "claude_code.api_request" } },
          { key: "prompt.id", value: { stringValue: "prompt-xyz" } },
          { key: "input_tokens", value: { intValue: "800" } },
          { key: "output_tokens", value: { intValue: "300" } },
          { key: "cache_read_tokens", value: { intValue: "0" } },
          { key: "cache_creation_tokens", value: { intValue: "0" } },
          { key: "cost_usd", value: { doubleValue: 0.009 } },
          { key: "duration_ms", value: { intValue: "1800" } },
          { key: "model", value: { stringValue: "claude-haiku-4-5" } },
        ],
      }],
    }],
  }],
};

describe("createOtlpEndpoint", () => {
  let server: ReturnType<typeof createOtlpEndpoint>;
  let port: number;

  beforeEach(() => {
    // Use random port to avoid conflicts
    port = 14400 + Math.floor(Math.random() * 100);
  });

  afterEach(() => {
    server?.stop();
  });

  it("returns 200 and logs event in dry-run mode (metrics format)", async () => {
    const logs: string[] = [];
    const consoleSpy = vi.spyOn(console, "log").mockImplementation((...args) => {
      logs.push(args.join(" "));
    });

    server = createOtlpEndpoint({ port, dryRun: true, repoId: "repo-1" });
    const res = await fetch(`http://localhost:${port}/v1/metrics`, {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify(METRICS_BODY),
    });

    expect(res.status).toBe(200);
    const body = await res.json();
    expect(body.received).toBe(1);
    expect(body.mode).toBe("dry-run");
    expect(logs.some(l => l.includes("prompt-abc"))).toBe(true);
    consoleSpy.mockRestore();
  });

  it("parses logs format (POST /v1/logs)", async () => {
    server = createOtlpEndpoint({ port, dryRun: true, repoId: "repo-1" });
    const res = await fetch(`http://localhost:${port}/v1/logs`, {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify(LOGS_BODY),
    });

    expect(res.status).toBe(200);
    const body = await res.json();
    expect(body.received).toBe(1);
  });

  it("ignores non-claude_code.api_request metrics", async () => {
    const otherBody = {
      resourceMetrics: [{
        scopeMetrics: [{
          metrics: [{ name: "some.other.metric", gauge: { dataPoints: [] } }],
        }],
      }],
    };
    server = createOtlpEndpoint({ port, dryRun: true, repoId: "repo-1" });
    const res = await fetch(`http://localhost:${port}/v1/metrics`, {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify(otherBody),
    });

    const body = await res.json();
    expect(body.received).toBe(0);
  });

  it("writes to supabase in write mode", async () => {
    const inserted: any[] = [];
    const mockClient = {
      from: (table: string) => ({
        insert: (row: any) => { inserted.push({ table, row }); return Promise.resolve({ error: null }); },
        select: () => ({ eq: () => ({ eq: () => ({ lte: () => ({ limit: () => Promise.resolve({ data: [], error: null }) }) }) }) }),
      }),
    };

    server = createOtlpEndpoint({ port, dryRun: false, repoId: "repo-1", supabaseClient: mockClient });
    await fetch(`http://localhost:${port}/v1/metrics`, {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify(METRICS_BODY),
    });

    expect(inserted).toHaveLength(1);
    expect(inserted[0].table).toBe("api_requests");
    expect(inserted[0].row.prompt_id).toBe("prompt-abc");
    expect(inserted[0].row.input_tokens).toBe(1000);
    expect(inserted[0].row.cost_usd).toBe(0.015);
  });

  it("returns 404 for unknown paths", async () => {
    server = createOtlpEndpoint({ port, dryRun: true, repoId: "repo-1" });
    const res = await fetch(`http://localhost:${port}/unknown`);
    expect(res.status).toBe(404);
  });
});
```

- [ ] **Step 2: Run tests to verify they fail**

```bash
cd packages/server && bunx vitest run src/otlp-endpoint.spec.ts
```
Expected: FAIL — `Cannot find module './otlp-endpoint.js'`

- [ ] **Step 3: Write `otlp-endpoint.ts`**

Key implementation notes:
- The `supabaseClient` passed in `mcp-entry.ts` is created by `makeSenseiClient()` which already sets `db: { schema: 'sensei' }` — call `.from("api_requests")` and `.from("task_sessions")` directly (no `.schema()` call needed).
- `intValue` in OTLP JSON is a string (e.g., `"1000"`) — coerce with `Number()`.
- Session correlation: find an `in_progress` task_session with `created_at <= recorded_at` and `(completed_at IS NULL OR completed_at >= recorded_at)`.

```typescript
// packages/server/src/otlp-endpoint.ts

export interface OtlpEvent {
  promptId: string;
  inputTokens: number;
  outputTokens: number;
  cacheReadTokens: number;
  cacheCreationTokens: number;
  costUsd: number;
  durationMs: number | null;
  model: string | null;
  recordedAt: string;
}

function getAttr(attrs: any[], key: string): any {
  const a = attrs?.find((x: any) => x.key === key);
  if (!a) return null;
  const v = a.value;
  return v.intValue ?? v.doubleValue ?? v.stringValue ?? null;
}

export function parseOtlpBody(body: any): OtlpEvent[] {
  const events: OtlpEvent[] = [];
  const now = new Date().toISOString();

  // Metrics format: resourceMetrics[].scopeMetrics[].metrics[].gauge.dataPoints[]
  for (const rm of body.resourceMetrics ?? []) {
    for (const sm of rm.scopeMetrics ?? []) {
      for (const m of sm.metrics ?? []) {
        if (m.name !== "claude_code.api_request") continue;
        const dataPoints = m.gauge?.dataPoints ?? m.sum?.dataPoints ?? [];
        for (const dp of dataPoints) {
          const attrs = dp.attributes ?? [];
          const promptId = getAttr(attrs, "prompt.id");
          if (!promptId) continue;
          events.push({
            promptId: String(promptId),
            inputTokens: Number(getAttr(attrs, "input_tokens") ?? 0),
            outputTokens: Number(getAttr(attrs, "output_tokens") ?? 0),
            cacheReadTokens: Number(getAttr(attrs, "cache_read_tokens") ?? 0),
            cacheCreationTokens: Number(getAttr(attrs, "cache_creation_tokens") ?? 0),
            costUsd: Number(getAttr(attrs, "cost_usd") ?? 0),
            durationMs: getAttr(attrs, "duration_ms") != null ? Number(getAttr(attrs, "duration_ms")) : null,
            model: getAttr(attrs, "model") ? String(getAttr(attrs, "model")) : null,
            recordedAt: now,
          });
        }
      }
    }
  }

  // Logs format: resourceLogs[].scopeLogs[].logRecords[]
  for (const rl of body.resourceLogs ?? []) {
    for (const sl of rl.scopeLogs ?? []) {
      for (const lr of sl.logRecords ?? []) {
        const attrs = lr.attributes ?? [];
        const eventName = getAttr(attrs, "event.name");
        if (eventName !== "claude_code.api_request") continue;
        const promptId = getAttr(attrs, "prompt.id");
        if (!promptId) continue;
        events.push({
          promptId: String(promptId),
          inputTokens: Number(getAttr(attrs, "input_tokens") ?? 0),
          outputTokens: Number(getAttr(attrs, "output_tokens") ?? 0),
          cacheReadTokens: Number(getAttr(attrs, "cache_read_tokens") ?? 0),
          cacheCreationTokens: Number(getAttr(attrs, "cache_creation_tokens") ?? 0),
          costUsd: Number(getAttr(attrs, "cost_usd") ?? 0),
          durationMs: getAttr(attrs, "duration_ms") != null ? Number(getAttr(attrs, "duration_ms")) : null,
          model: getAttr(attrs, "model") ? String(getAttr(attrs, "model")) : null,
          recordedAt: now,
        });
      }
    }
  }

  return events;
}

export interface OtlpEndpointOptions {
  port?: number;
  dryRun?: boolean;
  repoId: string;
  supabaseClient?: any;
}

export function createOtlpEndpoint(opts: OtlpEndpointOptions): { stop: () => void; port: number } {
  const port = opts.port ?? 4318;
  const dryRun = opts.dryRun ?? process.env.SENSEI_OTEL_DRY_RUN === "true";

  const server = Bun.serve({
    port,
    async fetch(req: Request) {
      const url = new URL(req.url);

      if (req.method === "POST" && (url.pathname === "/v1/metrics" || url.pathname === "/v1/logs")) {
        let body: any;
        try { body = await req.json(); }
        catch { return Response.json({ error: "Invalid JSON" }, { status: 400 }); }

        const events = parseOtlpBody(body);

        if (dryRun) {
          for (const event of events) {
            console.log("[sensei-otel dry-run]", JSON.stringify(event));
          }
          return Response.json({ ok: true, received: events.length, mode: "dry-run" });
        }

        if (opts.supabaseClient && events.length > 0) {
          for (const event of events) {
            // Correlate to active task session by time window
            const { data: sessions } = await (opts.supabaseClient as any)
              .from("task_sessions")
              .select("id")
              .eq("repo_id", opts.repoId)
              .eq("status", "in_progress")
              .lte("created_at", event.recordedAt)
              .limit(1);

            const taskSessionId = sessions?.[0]?.id ?? null;

            await (opts.supabaseClient as any)
              .from("api_requests")
              .insert({
                repo_id: opts.repoId,
                task_session_id: taskSessionId,
                prompt_id: event.promptId,
                input_tokens: event.inputTokens,
                output_tokens: event.outputTokens,
                cache_read_tokens: event.cacheReadTokens,
                cache_creation_tokens: event.cacheCreationTokens,
                cost_usd: event.costUsd,
                duration_ms: event.durationMs,
                model: event.model,
                recorded_at: event.recordedAt,
              });
          }
        }

        return Response.json({ ok: true, received: events.length });
      }

      return Response.json({ ok: false, error: "Not found" }, { status: 404 });
    },
  });

  const s = server as { stop: () => void; port: number };
  return { stop: () => s.stop(), port: s.port };
}
```

Note on the mock in the test for write mode: since `getDb()` sets `db: { schema: 'sensei' }` on the Supabase client, the mock only needs to implement `.from()` directly (no `.schema()` wrapper). The test mock above already reflects this.

- [ ] **Step 4: Run tests to verify they pass**

```bash
cd packages/server && bunx vitest run src/otlp-endpoint.spec.ts
```
Expected: 5 tests PASS

- [ ] **Step 5: TypeScript check**

```bash
bunx tsc --noEmit -p packages/server/tsconfig.json
```
Expected: zero errors

- [ ] **Step 6: Commit**

```bash
git add packages/server/src/otlp-endpoint.ts packages/server/src/otlp-endpoint.spec.ts
git commit -m "feat(server): add OTLP endpoint for Claude Code telemetry"
```

---

### Task 4: Wire OTLP endpoint into `mcp-entry.ts`

**Files:**
- Modify: `packages/server/src/mcp-entry.ts`

The MCP server uses stdio transport — it's launched as a subprocess by Claude Code. Adding a second HTTP server (the OTLP endpoint) on port 4318 alongside it lets Claude Code send telemetry to the same process. Note: `serve.ts` is the HTTP report server — the MCP entry point is `mcp-entry.ts`.

Current content of `packages/server/src/mcp-entry.ts`:

```typescript
#!/usr/bin/env bun
import { StdioServerTransport } from "@modelcontextprotocol/sdk/server/stdio.js";
import { createSenseiMcpServer } from "./mcp-server.js";
import { loadSenseiConfig } from "@sensei/shared";

const repoPath = process.env.SENSEI_REPO_PATH ?? process.cwd();
const config = await loadSenseiConfig(repoPath);

if (!config) {
  console.error("[sensei-mcp] No .sensei/config.yaml found. Run sensei init first.");
  process.exit(1);
}

const server = createSenseiMcpServer({ repoId: config.repo_id, repoPath });
const transport = new StdioServerTransport();
await server.connect(transport);
```

- [ ] **Step 1: Add OTLP endpoint startup to `mcp-entry.ts`**

Replace the full file with:

```typescript
#!/usr/bin/env bun
import { StdioServerTransport } from "@modelcontextprotocol/sdk/server/stdio.js";
import { createSenseiMcpServer } from "./mcp-server.js";
import { loadSenseiConfig, makeSenseiClient } from "@sensei/shared";
import { createOtlpEndpoint } from "./otlp-endpoint.js";

const repoPath = process.env.SENSEI_REPO_PATH ?? process.cwd();
const config = await loadSenseiConfig(repoPath);

if (!config) {
  console.error("[sensei-mcp] No .sensei/config.yaml found. Run sensei init first.");
  process.exit(1);
}

// Start OTLP endpoint for Claude Code telemetry
const otlpPort = parseInt(process.env.SENSEI_OTEL_PORT ?? "4318", 10);
const dryRun = process.env.SENSEI_OTEL_DRY_RUN === "true";
let supabaseClient: any = null;
try { supabaseClient = await makeSenseiClient(repoPath); } catch { /* no client — write mode silently skipped */ }

const otlp = createOtlpEndpoint({ port: otlpPort, dryRun, repoId: config.repo_id, supabaseClient });
console.error(`[sensei-otel] Listening on :${otlp.port} (${dryRun ? "dry-run" : "live"})`);

const server = createSenseiMcpServer({ repoId: config.repo_id, repoPath });
const transport = new StdioServerTransport();
await server.connect(transport);
```

- [ ] **Step 2: TypeScript check**

```bash
bunx tsc --noEmit -p packages/server/tsconfig.json
```
Expected: zero errors

- [ ] **Step 3: Run full server test suite**

```bash
bun run --filter '@sensei/server' test
```
Expected: zero failures

- [ ] **Step 4: Commit**

```bash
git add packages/server/src/mcp-entry.ts
git commit -m "feat(server): start OTLP endpoint alongside MCP server in mcp-entry"
```

---

## Chunk 3: Dashboard

### Task 5: Update `+page.server.ts` to include cost and benchmark data

**Files:**
- Modify: `apps/dashboard/src/routes/repos/[id]/analytics/+page.server.ts`

`getDb()` returns a Supabase client with `db: { schema: 'sensei' }` already set — call `.from()` directly, no `.schema("sensei")` prefix needed. Add two new queries after the existing `task_turns` aggregation, before the `return` statement.

- [ ] **Step 1: Add cost and benchmark queries**

In `apps/dashboard/src/routes/repos/[id]/analytics/+page.server.ts`, replace the `return { ... }` block at the end with the following (insert the new queries and update the return):

```typescript
  // Cost data from OTel api_requests — last 30 days
  const { data: apiRequestRows } = await db
    .from("api_requests")
    .select("task_session_id, input_tokens, output_tokens, cache_read_tokens, cache_creation_tokens, cost_usd, recorded_at")
    .eq("repo_id", params.id)
    .gte("recorded_at", since)
    .order("recorded_at", { ascending: false });

  // Aggregate cost per task_session_id client-side
  const costBySession = new Map<string, { inputTokens: number; outputTokens: number; cacheReadTokens: number; cacheCreationTokens: number; costUsd: number }>();
  for (const row of ((apiRequestRows ?? []) as Array<{ task_session_id: string | null; input_tokens: number; output_tokens: number; cache_read_tokens: number; cache_creation_tokens: number; cost_usd: string | number }>) ) {
    if (!row.task_session_id) continue;
    const existing = costBySession.get(row.task_session_id) ?? { inputTokens: 0, outputTokens: 0, cacheReadTokens: 0, cacheCreationTokens: 0, costUsd: 0 };
    costBySession.set(row.task_session_id, {
      inputTokens: existing.inputTokens + (row.input_tokens ?? 0),
      outputTokens: existing.outputTokens + (row.output_tokens ?? 0),
      cacheReadTokens: existing.cacheReadTokens + (row.cache_read_tokens ?? 0),
      cacheCreationTokens: existing.cacheCreationTokens + (row.cache_creation_tokens ?? 0),
      costUsd: existing.costUsd + Number(row.cost_usd ?? 0),
    });
  }

  // Benchmark runs — all time, paired by (task_description, branch)
  const { data: benchmarkRunRows } = await db
    .from("benchmark_runs")
    .select("id, task_description, branch, sensei_enabled, started_at, ended_at, total_cost_usd, total_input_tokens, total_output_tokens")
    .eq("repo_id", params.id)
    .order("started_at", { ascending: false });

  type BenchmarkEntry = { costUsd: number; inputTokens: number; outputTokens: number };
  type BenchmarkPair = {
    taskDescription: string;
    branch: string;
    withSensei: BenchmarkEntry | null;
    withoutSensei: BenchmarkEntry | null;
  };
  const pairMap = new Map<string, BenchmarkPair>();
  for (const run of ((benchmarkRunRows ?? []) as Array<{ task_description: string; branch: string; sensei_enabled: boolean; total_cost_usd: string | number | null; total_input_tokens: number | null; total_output_tokens: number | null }>) ) {
    const key = `${run.task_description}::${run.branch}`;
    const pair = pairMap.get(key) ?? { taskDescription: run.task_description, branch: run.branch, withSensei: null, withoutSensei: null };
    const entry: BenchmarkEntry = {
      costUsd: Number(run.total_cost_usd ?? 0),
      inputTokens: run.total_input_tokens ?? 0,
      outputTokens: run.total_output_tokens ?? 0,
    };
    if (run.sensei_enabled) pair.withSensei = entry;
    else pair.withoutSensei = entry;
    pairMap.set(key, pair);
  }
  const benchmarkPairs = Array.from(pairMap.values()).filter(p => p.withSensei !== null && p.withoutSensei !== null);

  return {
    repo: repo as { id: string; name: string },
    sessions,
    toolUsage,
    costBySession: Object.fromEntries(costBySession),
    benchmarkPairs,
  };
```

- [ ] **Step 2: TypeScript check**

```bash
bunx tsc --noEmit -p apps/dashboard/tsconfig.json 2>&1 | grep -v "node_modules" | head -20
```
Expected: zero new errors in the analytics files (pre-existing dashboard TS errors from env vars/deps are acceptable if they existed before this task)

- [ ] **Step 3: Commit**

```bash
git add apps/dashboard/src/routes/repos/[id]/analytics/+page.server.ts
git commit -m "feat(dashboard): add cost and benchmark queries to analytics page server"
```

---

### Task 6: Update `+page.svelte` to show Cost and Benchmark sections

**Files:**
- Modify: `apps/dashboard/src/routes/repos/[id]/analytics/+page.svelte`

The existing component uses inline `<style>` and the `@rokkit/ui` `Table` component. The two new sections follow the same style conventions. Insert after the closing `</div>` of the `.session-list` block (after the `{/if}` that closes the Task Sessions section).

- [ ] **Step 1: Add Cost section and Benchmark Comparison section**

Append the following before the closing `</style>` tag — first add the two new HTML sections after the existing task sessions `{/if}`, then extend `<style>`:

New Svelte sections (insert after `{/if}` that closes the Task Sessions block, before `<style>`):

```svelte
{#if Object.keys(data.costBySession).length > 0}
<h2>Cost — Last 30 Days</h2>
<table class="cost-table">
  <thead>
    <tr>
      <th>Session</th>
      <th class="num">Input tokens</th>
      <th class="num">Output tokens</th>
      <th class="num">Cache read</th>
      <th class="num">Cache write</th>
      <th class="num">Cost (USD)</th>
    </tr>
  </thead>
  <tbody>
    {#each data.sessions as session}
      {#if data.costBySession[session.id]}
        {@const cost = data.costBySession[session.id]}
        <tr>
          <td class="desc">{truncate(session.taskDescription)}</td>
          <td class="num mono">{cost.inputTokens.toLocaleString()}</td>
          <td class="num mono">{cost.outputTokens.toLocaleString()}</td>
          <td class="num mono">{cost.cacheReadTokens.toLocaleString()}</td>
          <td class="num mono">{cost.cacheCreationTokens.toLocaleString()}</td>
          <td class="num mono">${cost.costUsd.toFixed(4)}</td>
        </tr>
      {/if}
    {/each}
  </tbody>
</table>
{/if}

{#if data.benchmarkPairs.length > 0}
<h2>Benchmark Comparison</h2>
<table class="cost-table">
  <thead>
    <tr>
      <th>Task</th>
      <th>Branch</th>
      <th class="num">Without sensei</th>
      <th class="num">With sensei</th>
      <th class="num">Savings ($)</th>
      <th class="num">Savings (%)</th>
    </tr>
  </thead>
  <tbody>
    {#each data.benchmarkPairs as pair}
      {@const savings = (pair.withoutSensei?.costUsd ?? 0) - (pair.withSensei?.costUsd ?? 0)}
      {@const savingsPct = pair.withoutSensei?.costUsd ? (savings / pair.withoutSensei.costUsd) * 100 : 0}
      <tr>
        <td class="desc">{truncate(pair.taskDescription)}</td>
        <td class="mono" style="font-size:0.75rem">{pair.branch}</td>
        <td class="num mono">${pair.withoutSensei?.costUsd.toFixed(4)}</td>
        <td class="num mono">${pair.withSensei?.costUsd.toFixed(4)}</td>
        <td class="num mono" class:savings-pos={savings > 0} class:savings-neg={savings <= 0}>${savings.toFixed(4)}</td>
        <td class="num" class:savings-pos={savingsPct > 0} class:savings-neg={savingsPct <= 0}>{savingsPct.toFixed(1)}%</td>
      </tr>
    {/each}
  </tbody>
</table>
{/if}
```

New CSS to add inside the existing `<style>` block:

```css
  .cost-table { width: 100%; border-collapse: collapse; font-size: 0.875rem; margin-bottom: 8px; }
  .cost-table th { text-align: left; padding: 6px 12px 6px 0; border-bottom: 2px solid #e2e8f0; color: #64748b; font-weight: 600; font-size: 0.75rem; text-transform: uppercase; letter-spacing: 0.03em; }
  .cost-table td { padding: 8px 12px 8px 0; border-bottom: 1px solid #f1f5f9; }
  .cost-table tr:hover td { background: #f8fafc; }
  .cost-table .num { text-align: right; }
  .cost-table .mono { font-family: monospace; }
  .cost-table .desc { max-width: 300px; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
  .savings-pos { color: #166534; }
  .savings-neg { color: #991b1b; }
```

- [ ] **Step 2: Commit**

```bash
git add apps/dashboard/src/routes/repos/[id]/analytics/+page.svelte
git commit -m "feat(dashboard): add cost breakdown and benchmark comparison to analytics"
```

---

## Chunk 4: Skill Update

### Task 7: Update `skills/running-benchmarks/SKILL.md` with benchmark pair protocol

**Files:**
- Modify: `skills/running-benchmarks/SKILL.md`

The current SKILL.md covers corpus-level A/B runs with JSON result files. This task adds a new section for the OTel-based single-task benchmark pair protocol (with/without sensei, using worktrees and the OTLP endpoint). Insert the new section between the frontmatter/overview and the existing "## A/B Setup" section.

- [ ] **Step 1: Add the benchmark pair protocol section**

Insert the following after the "# Benchmark Runner\n\n## Overview\n\n..." paragraph (after the first prose paragraph ending with "...and task success rate."), before "## A/B Setup":

```markdown
## Benchmark Pair (with vs. without sensei)

Use this for a direct cost comparison on a single task using OTel telemetry. Results appear automatically in the repo's Analytics page under "Benchmark Comparison".

### Prerequisites

1. `CLAUDE_CODE_ENABLE_TELEMETRY=1` must be set in the shell environment
2. The sensei MCP server must be running (it hosts the OTLP endpoint on port 4318)
3. `SENSEI_OTEL_DRY_RUN` must be unset or `false` so costs are written to Supabase

### Procedure

**Step 1: Create a benchmark worktree**

```bash
SLUG=<short-task-name>
STAMP=$(date +%Y%m%d-%H%M)
git worktree add .worktrees/benchmark-${SLUG}-${STAMP} -b benchmark/${SLUG}-${STAMP}
cd .worktrees/benchmark-${SLUG}-${STAMP}
```

**Step 2: Run WITHOUT sensei**

1. Back up and strip sensei from the worktree's `.mcp.json`:
   ```bash
   cp .mcp.json .mcp.json.sensei-backup
   # Remove the sensei server entry from .mcp.json
   ```
2. Export OTel env vars:
   ```bash
   export CLAUDE_CODE_ENABLE_TELEMETRY=1
   export OTEL_METRICS_EXPORTER=otlp
   export OTEL_EXPORTER_OTLP_ENDPOINT=http://localhost:4318
   export SENSEI_OTEL_DRY_RUN=false
   ```
3. Insert a `benchmark_runs` row (sensei_enabled: false) — note the returned `id` as `$NO_SENSEI_RUN_ID`
4. Run the task with Claude Code using the exact task prompt
5. After completion: update the `benchmark_runs` row with `ended_at` and aggregate totals from `api_requests` rows where `recorded_at` is between `started_at` and `ended_at`

**Step 3: Run WITH sensei**

1. Restore `.mcp.json`:
   ```bash
   cp .mcp.json.sensei-backup .mcp.json && rm .mcp.json.sensei-backup
   ```
2. Same OTel env vars (already exported)
3. Insert a `benchmark_runs` row (sensei_enabled: true) — note the `id` as `$WITH_SENSEI_RUN_ID`
4. Run the exact same task prompt with Claude Code
5. Update the `benchmark_runs` row with `ended_at` and aggregate totals

**Step 4: Clean up**

```bash
cd ../..
git worktree remove .worktrees/benchmark-${SLUG}-${STAMP}
git branch -d benchmark/${SLUG}-${STAMP}
```

**Step 5: View results**

Open the repo's Analytics page in the sensei dashboard. The "Benchmark Comparison" table shows the paired runs side-by-side with cost delta and savings percentage. Pairs are matched by `(task_description, branch)`.

### What makes a good benchmark task

- Identical prompt for both runs — copy it verbatim, do not paraphrase
- Same branch/codebase state (hence the worktree approach)
- Task genuinely exercises sensei context tools (`context_pack`, `search`, `get_session_context`) — otherwise savings will be minimal
- Tasks in the 5–20 tool call range show the clearest variance; single-tool tasks are too noisy

### Environment variables reference

| Variable | Value | Purpose |
|---|---|---|
| `CLAUDE_CODE_ENABLE_TELEMETRY` | `1` | Enables OTel emission from Claude Code |
| `OTEL_METRICS_EXPORTER` | `otlp` | Routes metrics to OTLP HTTP |
| `OTEL_EXPORTER_OTLP_ENDPOINT` | `http://localhost:4318` | Points to sensei's OTLP endpoint |
| `SENSEI_OTEL_DRY_RUN` | `false` (or unset) | Enables DB writes; set to `true` to verify event parsing only |
| `SENSEI_OTEL_PORT` | `4318` (default) | Override OTLP port if 4318 is taken |
```

- [ ] **Step 2: Commit**

```bash
git add skills/running-benchmarks/SKILL.md
git commit -m "docs(skills): add OTel benchmark pair protocol to running-benchmarks"
```

---

## Final Verification

- [ ] **Run full test suite**

```bash
bun run --filter '*' test
```
Expected: zero failures

- [ ] **TypeScript check (server + CLI)**

```bash
bunx tsc --noEmit -p packages/server/tsconfig.json
bunx tsc --noEmit -p packages/cli/tsconfig.json
```
Expected: zero errors in server and CLI packages

- [ ] **Verify DDL files exist**

```bash
ls database/ddl/table/sensei/ | grep -E "api_requests|benchmark_runs"
```
Expected: `api_requests.ddl` and `benchmark_runs.ddl` listed

- [ ] **Verify migration files exist**

```bash
ls supabase/migrations/ | grep 202603170000
```
Expected: `20260317000000_pattern_usages.sql`, `20260317000001_benchmark_runs.sql`, `20260317000002_otel_api_requests.sql` all listed

- [ ] **Manual verification (dry-run)**

```bash
SENSEI_OTEL_DRY_RUN=true CLAUDE_CODE_ENABLE_TELEMETRY=1 OTEL_METRICS_EXPORTER=otlp OTEL_EXPORTER_OTLP_ENDPOINT=http://localhost:4318 claude
```
Run any Claude Code task. Expected: sensei MCP server stderr shows `[sensei-otel dry-run] {...}` lines with token counts and cost for each API call.
