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
, cadence           metric_cadence   not null default 'day'
, capture_source    metric_capture   not null default 'snapshot'
, weight            numeric          not null default 1
, target            numeric
, rating_scale      jsonb            -- 5 thresholds (improvement order) → a 0-5 rating; null = not rated (neutral/uncomputed). See docs/spec/2026-08-20-metric-rating-scales-health.md
-- Metric keys whose relationship to THIS metric is definitional or mechanical
-- rather than informative — the suppression list for correlation analysis.
-- Measured on real data, an unfiltered ranking is topped by arithmetic:
-- tokens_in_per_day vs tokens_per_day correlates 1.00 because the second
-- CONTAINS the first, and session_duration vs the token counts sits at 0.89-0.92
-- because a longer session mechanically consumes more. Presenting those as
-- insights would bury the genuine findings (spec_depth vs spec_deviation_rate at
-- -0.54, throughput vs shallow-analysis at 0.77). Symmetric by convention: list
-- the relationship on either side and the engine treats it both ways.
, derives_from      text[]
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
comment on column metrics.cadence
     is 'How the metric''s group advances against its watermark: commit (immutable, new commits only) vs day (calendar-day, reopens the trailing day). Groups are (repository, task_name, cadence) so a mixed-cadence task like churn splits (churn-rate/concentration=commit, rework_density=day).';
comment on column metrics.capture_source
     is 'Whether this metric AUTHORIZES the activity-pruner to reclaim a day''s raw sessions (invariant I20). session = session_outcomes/autonomy (authorizes); git = churn/quality; snapshot = knowledge/tool/health + rework_density (never authorize). Defaults to snapshot (fail-closed: a new metric never wrongly authorizes reclaim).';
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
