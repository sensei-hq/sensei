# 測 · Metrics — registry-driven computation, storage, and roll-up

**Feature:** [[features/metrics]] · per-metric detail [[features/metrics/catalog]]
| **Reads:** `activity.sessions` · `activity.turns` · `activity.task_executions` (+ `sensei.folder_path_aliases`) · `activity.assistant_events` · `activity.runs` · `sensei.nodes` · `sensei.memories` · `inference.detected_patterns` · `inference.corrections` · `sensei.assistant_tools` + `sensei.tool_call_verdicts` · *(blocked)* transcript usage + `gateway.model_prices`
| **Writes:** `sensei.metrics` · `sensei.project_metrics`
| **Supersedes:** `sensei.ftr_daily` · `sensei.project_ftr_metrics` (views — retired after consumers repoint; see [Supersede & retire](#supersede--retire-the-ftr-specific-views)) · the legacy `GET /api/metrics/{project}` handler
| **Owner files:** `crates/senseid/src/tasks/metrics_scheduler.rs` · `crates/senseid/src/tasks/handlers/metrics/*.rs` · `crates/senseid/src/db/pg_store.rs`
| **Scope:** metrics only. The session-retro → insights → dynamic skill/agent/rule
registry is a **separate facet** (see [Out of scope](#out-of-scope)). App surfacing
is a follow-up after db + api + tasks land.

## Purpose

Give every project a small set of measured, first-party, paired numbers and their
trends, computed on a schedule from data sensei already captures, so a developer
can see whether the pairing is improving and act on it. The metric set is
**data-driven**: a `metrics` registry says what to compute, how to describe it,
and which scheduled task produces it. Values are stored **once** in
`sensei.project_metrics` at project + session + date grain; all aggregation
(week / month / quarter / trend) and the derived **health score** are views over
that one store. **`project_metrics` becomes the single source of truth for FTR** —
the existing FTR-specific views are retired onto it (one north-star number, not
three). Nothing here fabricates a value.

## Supersede & retire the FTR-specific views

FTR is today computed three ways — `sensei.project_ftr_metrics` + `sensei.ftr_daily`
(views over `activity.sessions.ftr`, read by `pg_store::get_ftr_daily` and the
project 14d getter) and a separate ad-hoc `GET /api/metrics/{project}` handler
(`api/handlers/observatory.rs`, which computes `completed/session_count` and
returns a **fabricated `0`** on no data — the exact anti-pattern this spec bans).
`sessions.ftr` (the per-session boolean, set by the analyzer) **stays** — it is the
raw signal. What consolidates onto `project_metrics` is the *roll-up*:

1. The `session_outcomes` task writes FTR at daily grain into `project_metrics`
   with `props.session_count` + `props.correction_count`, matching what
   `ftr_daily` produced (and re-derivable to the 7d/14d windows `project_ftr_metrics`
   served).
2. **Repoint consumers** to read `project_metrics`: `pg_store::get_ftr_daily` + the
   14d-FTR getter `get_project_ftr` (`pg_store.rs:~7439`, headline path); the
   endpoints `/api/observatory/ftr-daily`, `/api/projects/{id}/ftr-daily`, and the
   **MCP `get_ftr_daily`** tool (keep the response shape — re-source only); the
   app's `api.ts` ftr-daily + `overview-view` 14d chip. **`measure_verdicts` needs
   no change** — it computes FTR inline from `activity.sessions` (`sessions.ftr`,
   which stays), not from these views. (`impact.md` is stale on this — a separate
   doc-fix; see the plan's follow-ups.)
3. Replace the fabricated-`0` FTR in BOTH the legacy `GET /api/metrics/{project}`
   handler AND the MCP `get_metrics` tool arm with the store-backed data.
4. **Only then** drop the `ftr_daily` + `project_ftr_metrics` view DDL and update
   `pipeline/ftr.md` + `pipeline/impact.md` to name `project_metrics` as the FTR
   source. Nothing is dropped before its consumers move (P4 blast-radius rule).

## Data invariants

New enum types (own DDL files under `database/ddl/enum/sensei/`):
`metric_family` (outcome·cost·velocity·quality·autonomy·knowledge·tool·composite),
`metric_type` (ratio·pct·count·duration·currency·value·score),
`metric_direction` (higher_better·lower_better·neutral),
`metric_grain` (session·daily), `metric_source` (measured·estimated).

### `sensei.metrics` — the registry (what to compute + how to read it)

A metric is **active** on a day when `day ∈ [effective_from, coalesce(effective_until,
'infinity'))`. Retirement = a past `effective_until`; `retire_reason` records why.

```sql
create table if not exists metrics (
    id              uuid          primary key default gen_random_uuid()
  , key             text          not null unique          -- stable slug: 'ftr', 'rework_ratio'
  , name            text          not null
  , description     text          not null
  , family          metric_family not null
  , type            metric_type   not null
  , unit            text                                    -- '%','tokens','$'; null for pure ratios
  , direction       metric_direction not null
  , purpose         text          not null                 -- what it tells you
  , how_to_read     text          not null                 -- how to interpret it (+ companion, gotcha)
  , formula         text          not null                 -- human-readable computation
  , task_name       text          not null                 -- maps to the TaskKind that computes it
  , weight          numeric       not null default 1        -- contribution to the health score
  , target          numeric                                 -- normalization bound for counts/durations in the health score
  , effective_from  date          not null default current_date
  , effective_until date                                    -- null = active; a past date = retired
  , retire_reason   text
  , modified_at     timestamptz   not null default now()
);
create index if not exists metrics_task_idx on metrics (task_name);
```

**Registry ↔ code contract (important — no dynamic dispatch).** `task_name` maps
to a **compiled `TaskKind`** variant (`crates/senseid/src/tasks/mod.rs`) with a
handler; there is no string-dispatch layer. So the registry makes metrics
**describable, enable/disable-able (by `effective_*`), retire-able, and
schedulable as data** — but a *genuinely new computation* still needs a new
`TaskKind` + handler (code). Consequently: **a blocked metric's row is NOT seeded
until its handler ships.** Its catalog entry exists (documentation), but the
staging→import step defers that row until the `task_name` handler exists, so the
scheduler can never enqueue a `task_name` with no handler.

Seeded from [`features/metrics/catalog.md`](../../features/metrics/catalog.md) via
the **staging + import procedure** pattern (precedent:
`database/ddl/table/staging/scopes.ddl` + `database/ddl/procedure/staging/import_scopes.ddl`):
a `staging.metrics` table loaded from a data file, then `staging.import_metrics(...)`
upserts into `sensei.metrics`, timestamp-guarded (incremental, never clobbers
edited rows). Only rows whose `task_name` handler exists are included.

### `sensei.project_metrics` — the value store (generalized, single source)

```sql
create table if not exists project_metrics (
    id           uuid          primary key default gen_random_uuid()
  , metric_id    uuid          not null references sensei.metrics(id) on delete cascade
  , project_id   uuid          not null references sensei.projects(id) on delete cascade
  , folder_id    uuid          references sensei.folders(id) on delete cascade  -- per-module; null = whole project
  , session_id   uuid                                                           -- per-session grain; null = aggregate
  , computed_on  date          not null                                         -- the date the value is FOR
  , grain        metric_grain  not null                                         -- session | daily
  , value        numeric       not null
  , props        jsonb         not null default '{}'                            -- numerator/denominator, session_count, low_n, evidence ids
  , source       metric_source not null default 'measured'
  , modified_at  timestamptz   not null default now()
);
create unique index if not exists project_metrics_identity
    on project_metrics (metric_id, project_id, folder_id, session_id, computed_on, grain)
    nulls not distinct;
create index if not exists project_metrics_lookup
    on project_metrics (project_id, metric_id, computed_on);
```

Invariants (never-fabricate, applied):
- **No data ⇒ no row.** Absence reads as "not yet measured," never a defaulted `0`.
- **Grain explicit.** `grain='session'` ⇒ `session_id` set; `grain='daily'` ⇒
  `session_id` null. `folder_id` set only for module-scoped metrics.
- **Ratios carry their parts.** A ratio/pct row stores `props.numerator` +
  `props.denominator` so roll-ups re-derive, never average-of-averages.
- **Estimates tagged.** `source='estimated'` never rendered as truth; money-facing
  metrics write no row on a price miss (fail closed).
- Preconditions: the registry is seeded; `activity.sessions` has ≥1 enriched
  session for the project in the window.

### Aggregation views (DDL files under `database/ddl/view/sensei/`)

Lowercase keywords, leading commas, `set search_path` header, `comment on view` —
style exemplar: `database/ddl/view/sensei/ftr_daily.ddl`. Names (no `v_` prefix):
`project_metric_daily` (base), `project_metric_weekly`, `_monthly`, `_quarterly`,
`project_metric_trend`, `project_health`.

```sql
-- project_metric_daily.ddl  (base: project × metric × date, project scope)
set search_path to sensei, extensions;

create or replace view project_metric_daily
    as
select pm.project_id
     , m.key          as metric
     , pm.computed_on as date
     , pm.value
     , pm.props
     , m.type
     , m.direction
  from sensei.project_metrics pm
  join sensei.metrics         m
    on m.id           = pm.metric_id
 where pm.grain      = 'daily'
   and pm.folder_id is null;
```

Coarser grains `date_trunc` the base and aggregate **by type, inline** (no opaque
helper): ratios/pcts re-derive `sum(numerator)/nullif(sum(denominator),0)`,
counts/currency `sum(value)`, point-in-time `value`/`score` take the period end
via `(array_agg(value order by date desc))[1]`:

```sql
-- project_metric_weekly.ddl
set search_path to sensei, extensions;

create or replace view project_metric_weekly
    as
select project_id
     , metric
     , date_trunc('week', date)::date as period
     , case
         when type in ('ratio', 'pct')
           then sum((props->>'numerator')::numeric)
                / nullif(sum((props->>'denominator')::numeric), 0)
         when type in ('count', 'currency')
           then sum(value)
         else (array_agg(value order by date desc))[1]
       end                            as value
     , direction
  from sensei.project_metric_daily
 group by project_id, metric, period, type, direction;
-- _monthly / _quarterly: identical with 'month' / 'quarter'.
```

`project_metric_trend` adds the prior period + delta via `lag()` over the weekly
view (for the arrow). `project_health` is the derived score (below). No view
averages averages.

### `project_health` — the derived health score

A single 0–100 score per project per date, rolled from the **active** metrics:
each metric's latest daily value is normalized to [0,1] by its `direction`
(`higher_better` → v; `lower_better` → 1−v; ratios/pcts are already bounded;
counts/durations normalize against `metrics.target`, and are excluded when
`target is null`), combined by `metrics.weight`, ×100. It is itself a registered
metric (`key='project_health'`, `family='composite'`, `type='score'`,
`direction='higher_better'`, `task_name` → `TaskKind::ComputeHealth`) so it stores,
trends, and renders like any other. Written only when ≥1 component has a value
(never a fabricated 100 on an empty project).

## What's computed + how (the registry-driven scheduler)

### The scheduler

A daily `ComputeProjectMetrics` scheduler (sibling of
`crates/senseid/src/tasks/analyzer_scheduler.rs`, using the same persisted
`sensei.config` watermark convention; window = `metrics.window_days`, default 14):

1. Read the **active** registry:
   `select distinct task_name from sensei.metrics
      where effective_from <= current_date
        and (effective_until is null or effective_until > current_date)`.
   (Blocked metrics aren't seeded, so their task_name never appears.)
2. For each project × distinct active `task_name`, enqueue that compute task over
   the rolling window; capture the enqueued task ids.
3. Enqueue `ComputeHealth` **blocked_by those task ids** — reusing the task
   queue's `depends_on` barrier (the same mechanism the scan pipeline uses), so
   health runs only after the base rows for the day exist. No ad-hoc waiting.
4. Each task recomputes session + date grain for the window and **upserts**
   (idempotent; `on conflict` on the identity index; `modified_at = now()`). A
   re-run backfills gaps, never duplicates.
5. A metric past its `effective_until` is excluded from step 1; its history is kept
   for audit unless purged.

### task_name → TaskKind groups (a task reads a source where possible and computes several metrics)

Grouping principle: a `task_name` computes the metrics that **share a source where
possible** (so each source is read once); a task may read more than one table when
a family naturally spans them. v1:

| `task_name` → `TaskKind` | Metrics | Reads |
|---|---|---|
| `session_outcomes` | ftr, rework_ratio, throughput | `sessions`, `turns` |
| `churn` | churn_rate, churn_concentration, rework_density | `task_executions` (→ folder join), `detected_patterns` |
| `duplication` | duplication_ratio | `nodes` (via `find_duplicates_scoped`) |
| `autonomy` | interruption_rate, false_crash_rate, run_completion | `assistant_events`, `runs` |
| `knowledge` | memory_promotion | `memories`, `detected_patterns`, `corrections` |
| `tool` | unused_tools | `assistant_tools`, `tool_call_verdicts` |
| `health` | project_health (derived) | `project_metrics` (the day's rows) |
| `cost` *(blocked — not seeded)* | tokens_per_session, cost_of_rework, cache_hit_ratio | transcript usage + `gateway.model_prices` |

Notes:
- **`reopen_rate` is blocked, not v1** — `activity.turns` has no file/module column,
  so cross-session reopen can't be attributed yet; its catalog row is not seeded.
- **`churn` folder attribution:** `activity.task_executions` has no project/folder
  id — only `folder_path`. Resolve `folder_path → sensei.folders.abs_path →
  folder_id/project_id`, applying `sensei.folder_path_aliases` for renamed folders,
  before writing project/module-scoped rows.
- **Module-scoped (folder_id set) v1 metrics:** `rework_density`, `churn_rate`,
  `duplication_ratio` write per-module rows (for the module × metric heatmap) *and*
  the project aggregate; `ftr`, `rework_ratio`, `throughput`, autonomy, knowledge,
  health are project-scoped only. (The catalog's Facets are updated to match.)
- **`duplication` reuses `PgStore::find_duplicates_scoped`** (the internal fn behind
  the `get_duplicates` MCP tool / `/api/patterns/{project}/duplicates`) — called
  directly, not over HTTP (DRY).

### Read endpoints

- `GET /api/metrics/registry` → the active catalog (key, family, type, direction,
  purpose, how_to_read) — powers self-describing UI.
- `GET /api/projects/{id}/metrics` → latest per metric + trend + health.
- `GET /api/projects/{id}/metrics/{key}?grain=daily|weekly|monthly|quarterly` → the series.
- The legacy `GET /api/metrics/{project}` (ad-hoc, fabricates `0`) is **replaced**
  by these; `get_ftr_daily` + the FTR endpoints/MCP tool are re-sourced from
  `project_metrics` (shape unchanged). Catalog columns travel with each value so
  the UI renders "what it tells you / is this good" without hardcoding.

## Done gate

- Registry seeded + self-describing: `curl -s localhost:7744/api/metrics/registry |
  jq '[.[]|select(.purpose==null or .direction==null)]|length'` → `0`.
- After a run, the sensei project has a daily **ftr** value with `props.session_count`
  + `props.correction_count`, its companion **rework_ratio**, and a **project_health**
  in 0–100 — all `source=measured`.
- FTR served from the store matches the (repointed) `/api/projects/{id}/ftr` and
  `/ftr-daily` — one number, no drift (the ftr.md Wrong-gate class stays closed).
- The weekly view re-derives a ratio from numerator/denominator (3 of 4 → 0.75,
  not the mean of daily ratios).
- Setting a metric's `effective_until` to yesterday stops new rows next run; history
  remains. `ComputeHealth` does not run until its `depends_on` base tasks complete.
- A metric with no data for a project/day produces no row.
- After repoint, `ftr_daily` + `project_ftr_metrics` are dropped and no code
  references them (`grep` clean).

## Wrong gate

- FTR computed/served **more than one way** — a divergence between the store and
  `/api/projects/{id}/ftr` (the three-way conflict this spec exists to end).
- A weekly/monthly value that is the **arithmetic mean of daily ratios**.
- A **fabricated `0`** where data is absent (incl. the legacy `/api/metrics`
  handler surviving), or a health score for a project with no components.
- A `task_name` enqueued with **no matching `TaskKind` handler** (a blocked metric
  seeded early), or a metric past `effective_until` still computed.
- **Cost** written with a defaulted price on a miss instead of failing closed.
- `churn`/`rework_density` rows attributed to the **wrong project** (folder_path
  alias resolution skipped) or not attributed (never resolve).
- `grain='daily'` rows carrying a `session_id`; per-module rows leaking into the
  project-scope base view (`folder_id is null` filter missing).
- FTR shown without its rework companion for the same scope/window.
- `ftr_daily`/`project_ftr_metrics` dropped **before** consumers repoint (a 500 on
  `/ftr-daily`).

## Out of scope

The **session-retro → insights → action-items → dynamic per-project registry of
skills / agents / memories / rules** is a **separate facet** (note:
`crates/senseid/src/tasks/handlers/session_retro.rs` already exists — that facet is
partly built). It reuses this spec's *shape* — a registry + scheduled tasks +
generated artifacts — but a learned artifact requires a **minimum threshold**
(never from a single session; existing thresholds to be revisited). It consumes
these metrics as its evidence layer and gets its own spec.

**Surfacing** in the sensei + dōjō apps (overview/impact chips, trends, the module
heatmap, the health dial) is a follow-up once db + api + tasks land.

## Related

- [[features/metrics]] · [[features/metrics/catalog]] · [[pipeline/ftr]] (FTR
  source moves here) · [[pipeline/impact]] (unaffected — `measure_verdicts` reads
  `sessions.ftr` inline; that doc is stale, separate fix) ·
  [[pipeline/signals]] · [[pipeline/analyzer]] (scheduler sibling) ·
  [[analysis/2026-08-04-metrics-catalog]] ·
  [[blueprints/2026-08-06-project-quality-metrics]] (per-module quality family)
