set search_path to sensei, extensions;

-- Which catalogue metrics may be a target (docs/spec/dojo/targets-and-gates.md
-- §TG1), how each is read, and when its reading is too thin. Its own table and
-- import (import/staging/metric_gating.jsonl) so the catalogue's own seed
-- (metrics.jsonl) is never edited for it. Holds no direction or unit: those are
-- the catalogue's (sensei.metrics), and the comparator is derived from direction.
create table if not exists metric_gating (
  metric_id     uuid        primary key references sensei.metrics(id) on delete cascade
, eligible      boolean     not null
, delta_scoped  boolean     not null
, min_coverage  numeric     not null check (min_coverage > 0 and min_coverage <= 1)
, note          text        not null
, modified_at   timestamptz not null default now()
);

comment on table metric_gating
     is 'Target eligibility per catalogue metric (TG1). A metric with no row is not eligible. A neutral metric (no direction) is never eligible; staging.import_metric_gating refuses to make one so.';
comment on column metric_gating.metric_id
     is 'The catalogue metric (sensei.metrics.id) this row gates.';
comment on column metric_gating.eligible
     is 'Whether the metric may be a target. Session-behaviour metrics (FTR, rework, throughput, …) are insights, never targets.';
comment on column metric_gating.delta_scoped
     is 'true: the reading is what a change introduced; false: an absolute reading of the repository, which a target can ratchet.';
comment on column metric_gating.min_coverage
     is 'The share of the measured scope a reading must cover to count (0 < x <= 1): 0.5 for coverage (guarded files seen), 1.0 for graph-derived readings (the index complete).';
comment on column metric_gating.note
     is 'Why the metric is or is not eligible, in one sentence.';
comment on column metric_gating.modified_at
     is 'When the row last changed; the import only overwrites a row with a staging row at least this new.';

-- The dōjō Worker reads it (service_role) and signed-in people may read it; the
-- daemon connects as the owner. Same grants and read policy as sensei.metrics.
grant select on metric_gating to authenticated, service_role;
alter table metric_gating enable row level security;
drop policy if exists metric_gating_read on metric_gating;
create policy metric_gating_read on metric_gating for select to authenticated using (true);
