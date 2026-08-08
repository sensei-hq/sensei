set search_path to sensei, extensions;
create table if not exists metrics (
  id                uuid             primary key default gen_random_uuid()
, key               text             not null unique
, name              text             not null
, description       text             not null
, family            metric_family    not null
, type              metric_type      not null
, unit              text
, direction         metric_direction not null
, purpose           text             not null
, how_to_read       text             not null
, formula           text             not null
, task_name         text             not null
, weight            numeric          not null default 1
, target            numeric
, effective_from    date             not null default current_date
, effective_until   date
, retire_reason     text
, modified_at       timestamptz      not null default now()
);

create index if not exists metrics_task_idx
    on metrics (task_name);

comment on table metrics is
'Metric registry — the data-driven catalog of what to compute, how to describe it,
and which scheduled task produces it. Seeded from features/metrics/catalog.md via the
staging + import procedure pattern (only rows whose task_name handler exists).

A metric is ACTIVE on a day when day is in [effective_from, coalesce(effective_until,
''infinity'')). Retirement = a past effective_until; retire_reason records why. A
genuinely new computation still needs a compiled TaskKind + handler (task_name maps to
a variant — no string dispatch), so a blocked metric is not seeded until its handler
ships. Values live in sensei.project_metrics; all roll-ups and the health score are
views over that one store.';

comment on column metrics.key
     is 'Stable slug used by code and views (e.g. ''ftr'', ''rework_ratio''). Unique.';
comment on column metrics.name
     is 'Human-readable display name.';
comment on column metrics.description
     is 'Short description of the metric.';
comment on column metrics.family
     is 'Metric family — the UI groups/colours from this, never a hardcoded name.';
comment on column metrics.type
     is 'Value type — drives how roll-ups aggregate (ratio/pct re-derive, count/currency sum, value/score take period end).';
comment on column metrics.unit
     is 'Display unit (''%'', ''tokens'', ''$''); null for pure ratios.';
comment on column metrics.direction
     is 'Which way is better — used to normalize the value in the health score.';
comment on column metrics.purpose
     is 'What the metric tells you.';
comment on column metrics.how_to_read
     is 'How to interpret it, including its companion metric and the gotcha that makes it lie if read alone.';
comment on column metrics.formula
     is 'Human-readable computation.';
comment on column metrics.task_name
     is 'Maps to the compiled TaskKind that computes this metric. Indexed (metrics_task_idx) for the scheduler''s distinct-active-task_name read.';
comment on column metrics.weight
     is 'Contribution to the composite health score.';
comment on column metrics.target
     is 'Normalization bound for counts/durations in the health score; null excludes the metric from the score.';
comment on column metrics.effective_from
     is 'First day the metric is active (inclusive). Defaults to current_date.';
comment on column metrics.effective_until
     is 'Last-active boundary (exclusive). Null = active; a past date = retired.';
comment on column metrics.retire_reason
     is 'Why the metric was retired (set alongside a past effective_until).';
comment on column metrics.modified_at
     is 'Timestamp of the last modification to this row.';
