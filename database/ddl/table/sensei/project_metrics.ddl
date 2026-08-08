set search_path to sensei, extensions;
create table if not exists project_metrics (
  id           uuid          primary key default gen_random_uuid()
, metric_id    uuid          not null references sensei.metrics(id) on delete cascade
, project_id   uuid          not null references sensei.projects(id) on delete cascade
, folder_id    uuid          references sensei.folders(id) on delete cascade
, session_id   uuid
, computed_on  date          not null
, grain        metric_grain  not null
, value        numeric       not null
, props        jsonb         not null default '{}'
, source       metric_source not null default 'measured'
, modified_at  timestamptz   not null default now()
);

-- Identity of a stored value: one row per metric x project x (module) x (session) x
-- date x grain. NULLS NOT DISTINCT so folder_id/session_id nulls (project-scope /
-- daily-grain rows) collide rather than duplicate — the upsert target the compute
-- tasks conflict on (idempotent re-run backfills, never duplicates).
create unique index if not exists project_metrics_identity
    on project_metrics (metric_id, project_id, folder_id, session_id, computed_on, grain)
    nulls not distinct;

create index if not exists project_metrics_lookup
    on project_metrics (project_id, metric_id, computed_on);

comment on table project_metrics is
'The value store — the single source of truth for every computed metric value, at
project + (module) + (session) + date grain. All aggregation (week/month/quarter/
trend) and the derived health score are views over this one store.

Never-fabricate invariants:
  - No data => no row. Absence reads as "not yet measured," never a defaulted 0.
  - Grain explicit. grain=''session'' => session_id set; grain=''daily'' => session_id
    null. folder_id set only for module-scoped metrics; null = whole project.
  - Ratios carry their parts. A ratio/pct row stores props.numerator + props.denominator
    so roll-ups re-derive (never average-of-averages).
  - Estimates tagged. source=''estimated'' is never rendered as truth; money-facing
    metrics write no row on a price miss (fail closed).';

comment on column project_metrics.metric_id
     is 'FK to sensei.metrics — which registered metric this value is for.';
comment on column project_metrics.project_id
     is 'FK to sensei.projects — the project the value belongs to.';
comment on column project_metrics.folder_id
     is 'FK to sensei.folders for a module-scoped (per-module) value. Null = whole project.';
comment on column project_metrics.session_id
     is 'Session this value is for, when grain=''session''. Null for aggregate (daily) rows.';
comment on column project_metrics.computed_on
     is 'The date the value is FOR (not when it was computed).';
comment on column project_metrics.grain
     is 'Grain of the row: session | daily.';
comment on column project_metrics.value
     is 'The computed value. Its meaning is defined by metrics.type/unit/direction.';
comment on column project_metrics.props
     is 'Extensible parts: numerator/denominator, session_count, correction_count, low_n, evidence ids.';
comment on column project_metrics.source
     is 'Provenance: measured | estimated. estimated is never rendered as truth.';
comment on column project_metrics.modified_at
     is 'Timestamp of the last modification to this row (set to now() on each upsert).';
