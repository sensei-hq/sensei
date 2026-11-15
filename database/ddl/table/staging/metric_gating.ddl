set search_path to staging, extensions;

-- Staging landing for import/staging/metric_gating.jsonl: flat, keyed by the
-- catalogue key. staging.import_metric_gating() resolves each key to its
-- sensei.metrics row and upserts sensei.metric_gating.
drop table if exists metric_gating cascade;
create table metric_gating (
  metric_key    text
, eligible      boolean
, delta_scoped  boolean
, min_coverage  numeric
, note          text
, modified_at   timestamptz
);

comment on table metric_gating
     is 'Staging rows for sensei.metric_gating, one per catalogue key (import/staging/metric_gating.jsonl).';
comment on column metric_gating.metric_key
     is 'The catalogue key (sensei.metrics.key). An unknown or missing key fails the import.';
comment on column metric_gating.eligible
     is 'Whether the metric may be a target.';
comment on column metric_gating.delta_scoped
     is 'true: what a change introduced; false: an absolute reading.';
comment on column metric_gating.min_coverage
     is 'The share of the measured scope a reading must cover to count.';
comment on column metric_gating.note
     is 'Why the metric is or is not eligible.';
comment on column metric_gating.modified_at
     is 'The row''s edit time; a live row newer than this is not overwritten.';

-- Row level security (the Supabase advisor), with no client policy. An import
-- staging table: written and read by the import alone, as the owner; a client
-- role reads nothing here. Inert on the daemon, which connects as the owner.
alter table staging.metric_gating enable row level security;
