set search_path to sensei, extensions;

-- Cached per-tool insight snapshot. The AggregateToolInsights task writes
-- one row per tool per run (append-only history); readers pick the latest
-- row per tool. Keeping the history makes the observatory
-- future-proof for trend charts without re-running the derivation.
--
-- `metrics` holds raw aggregates the derivation computes (call_count,
-- error_count, avg_duration_ms, last_used_at, error_rate). `signal_variant`
-- + `signal_title` + `signal_detail` capture the current SignalCard
-- verdict — the daemon's tool_signals module is the source of truth for
-- the rules, this table just caches its output. NULL variant = no signal
-- (tool is healthy + moderate use).
create table if not exists tool_insights (
  id             uuid            primary key default gen_random_uuid()
, tool_name      text            not null
, computed_at    timestamptz     not null default now()
, metrics        jsonb           not null default '{}'
, signal_variant text
, signal_title   text
, signal_detail  text
);

-- Hot path: "latest row per tool" — the reader orders by computed_at desc
-- inside a partitioned subquery, so we want the compound index leading with
-- tool_name for cheap seeks.
create index if not exists tool_insights_tool_time_idx
    on tool_insights(tool_name, computed_at desc);

-- Cold path: dashboards / trend charts that pull "everything in the last N
-- days" straight off the timeline.
create index if not exists tool_insights_time_idx
    on tool_insights(computed_at desc);

comment on table tool_insights is
'Cached per-tool insight snapshots produced by the AggregateToolInsights
scheduled task. Append-only — each run writes a fresh row per known tool.
The observatory tool-signals endpoint picks the latest row per tool
(via a distinct-on / row_number window) and returns them ordered by
variant priority. Cache-miss (no rows yet) falls back to the on-the-fly
derivation.';

comment on column tool_insights.id
     is 'Surrogate primary key (UUID).';
comment on column tool_insights.tool_name
     is 'Tool being insighted — matches assistant_events.tool_name.';
comment on column tool_insights.computed_at
     is 'When this snapshot was written.';
comment on column tool_insights.metrics
     is 'Raw aggregates: call_count, error_count, avg_duration_ms, last_used_at, error_rate.';
comment on column tool_insights.signal_variant
     is 'Signal card variant (warn / opportunity / unused / win) or NULL when no signal fires.';
comment on column tool_insights.signal_title
     is 'Card title copy (matches derive_signals output).';
comment on column tool_insights.signal_detail
     is 'Card detail copy (matches derive_signals output).';
