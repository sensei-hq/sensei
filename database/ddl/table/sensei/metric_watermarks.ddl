set search_path to sensei, extensions;

-- Per-(repository, metric_group) compute watermark — the coverage cursor for the
-- repo-grain metric engine (D4). The watermark IS coverage, so it RETIRES the
-- planner's covered_days / effective_from bookkeeping and the global
-- metrics.last_run clock. A day-cadence group records how far its calendar days are
-- settled in `sealed_through` (today is never sealed — it reopens as late sessions
-- land; late data reopens back to the earliest affected day). A commit-cadence group
-- (churn / quality) records the last commit it walked in `last_sha` and only ever
-- processes newer commits. The engine advances a group's watermark ONLY when that
-- group's compute succeeds, so a failed group holds its cursor and retries the same
-- range on the next run (fail-closed: a failing group never silently skips days).
create table if not exists metric_watermarks (
  repository_id uuid        not null references sensei.repositories(id) on delete cascade
, metric_group  text        not null
, sealed_through date
, last_sha       text
, updated_at    timestamptz not null default now()
, primary key (repository_id, metric_group)
);

comment on table metric_watermarks is
'Per-(repository, metric_group) compute watermark — the coverage cursor that retires
covered_days/effective_from + the global metrics.last_run (D4). day-cadence groups
seal calendar days in sealed_through (today never sealed; late data reopens);
commit-cadence groups (churn/quality) track the last walked commit in last_sha.
Advanced only on a group''s successful compute — a failed group holds its cursor and
retries (fail-closed, never a silent skip). Cascades on a repository delete.';

comment on column metric_watermarks.repository_id
     is 'The repository this watermark tracks (FK sensei.repositories, ON DELETE CASCADE).';
comment on column metric_watermarks.metric_group
     is 'The compute group (registry task_name + cadence, e.g. ''session_outcomes'', ''churn'') this cursor is for. Free text — groups are code-defined.';
comment on column metric_watermarks.sealed_through
     is 'Day-cadence: the last calendar date the group is settled through (today is never sealed). NULL until the first run.';
comment on column metric_watermarks.last_sha
     is 'Commit-cadence (churn/quality): the last commit sha the group walked through. NULL for day-cadence groups / until the first run.';
comment on column metric_watermarks.updated_at
     is 'Timestamp of the last watermark advance.';
