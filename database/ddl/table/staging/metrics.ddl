set search_path to staging, extensions;

-- Staging landing for the metric-registry seed (import/staging/metrics.jsonl).
-- Flat columns mirroring the datafile; the enum facets are text here and
-- import_metrics() casts them to their sensei.<enum> types on the way into
-- sensei.metrics, with a timestamp guard so a re-import is incremental and never
-- clobbers an edited row (same pattern as staging.rule_packs / staging.scopes).
--
-- effective_until + retire_reason carry a metric's RETIREMENT declaratively: the
-- datafile sets them (with a bumped modified_at) and import_metrics() applies them
-- onto the live row, so a re-seed can retire a metric in place — it never hand-
-- deletes a row. Null effective_until = active; a past/today date = retired (the
-- half-open [effective_from, effective_until) window that sensei.metrics documents).
drop table if exists metrics cascade;
create table metrics (
  key             text
, name            text
, description     text
, family          text        -- cast to sensei.metric_family on import
, type            text        -- cast to sensei.metric_type on import
, unit            text
, direction       text        -- cast to sensei.metric_direction on import
, purpose         text
, how_to_read     text
, formula         text
, task_name       text
, cadence         text        -- cast to sensei.metric_cadence on import
, capture_source  text        -- cast to sensei.metric_capture on import
, weight          numeric
, target          numeric
, effective_until date        -- null = active; a past/today date = retired
, retire_reason   text        -- why (set alongside a past effective_until)
, modified_at     timestamptz not null default now()
);
