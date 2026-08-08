set search_path to staging, extensions;

-- Staging landing for the metric-registry seed (import/staging/metrics.jsonl).
-- Flat columns mirroring the datafile; the enum facets are text here and
-- import_metrics() casts them to their sensei.<enum> types on the way into
-- sensei.metrics, with a timestamp guard so a re-import is incremental and never
-- clobbers an edited row (same pattern as staging.rule_packs / staging.scopes).
drop table if exists metrics cascade;
create table metrics (
  key           text
, name          text
, description   text
, family        text        -- cast to sensei.metric_family on import
, type          text        -- cast to sensei.metric_type on import
, unit          text
, direction     text        -- cast to sensei.metric_direction on import
, purpose       text
, how_to_read   text
, formula       text
, task_name     text
, weight        numeric
, target        numeric
, modified_at   timestamptz not null default now()
);
