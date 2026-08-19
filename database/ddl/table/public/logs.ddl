create table if not exists public.logs (
  id                       uuid        primary key default gen_random_uuid()
, level                    varchar     not null
, running_on               varchar     not null
, module                   varchar
, logged_at                timestamptz not null
, message                  varchar
, context                  jsonb       not null default '{}'
, data                     jsonb
, error                    jsonb
, written_at               timestamptz not null default now()
);

create index if not exists logs_level_idx
    on public.logs(level);

create index if not exists logs_running_on_idx
    on public.logs(running_on);

create index if not exists logs_logged_at_idx
    on public.logs(logged_at);

create index if not exists logs_module_idx
    on public.logs(module);

comment on table public.logs is
'Generic structured log table — shared across daemon, CLI, MCP, and app.
Follows the kavach logger pattern: level, running_on, context jsonb, data jsonb, error jsonb.

running_on: daemon, cli, mcp, app — identifies which component wrote the log.
module: top-level component bucket (tasks, api, indexer, gateway) — a first-class
  indexed column (like running_on), NOT a context prop, so it queries/indexes plainly.
context: flexible jsonb bag — callers inject attributes like method, task_id, tool, etc.
data: structured payload — task metrics (duration_ms, items_processed), API params, etc.
error: structured error — message, stack, code.

Common queries:
  -- All task executions sorted by duration
  SELECT * FROM logs WHERE module = ''tasks'' ORDER BY (data->>''duration_ms'')::int DESC
  -- Errors in the last hour
  SELECT * FROM logs WHERE level = ''error'' AND logged_at > now() - interval ''1h''
  -- All activity from a specific component
  SELECT * FROM logs WHERE running_on = ''mcp'' ORDER BY logged_at DESC';
