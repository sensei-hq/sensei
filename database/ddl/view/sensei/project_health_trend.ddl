set search_path to sensei, extensions;

-- Composite project health over TIME — the bar chart + trend line series, and the
-- radar's spokes for any historical period.
--
-- Derived, never stored. The score is an INTERPRETATION of the metric facts
-- (weights × rating scales), not an event that happened, so it is computed on
-- read. Retuning a weight or a scale — spec P-C is explicitly ongoing tuning —
-- re-reads all of history under the current model with no recompute-and-backfill
-- job, and no stored score can drift out of step with the inputs it came from.
--
-- `health_score = round(20 × Σ(weight·rating) / Σ(weight))` over the RATED metrics
-- of that period: rating is 0-5, so ×20 lands on 0-100 (spec R4). Identical shape
-- to `project_health_score` (the current, latest-only roll-up) because both read
-- the same ratings — this one just keeps the period dimension instead of
-- collapsing to the newest reading.
--
-- `having sum(weight) > 0` is the honest-empty rule (spec R6 / I4): a period with
-- nothing rated gets NO row rather than a fabricated 0, so a gap in the chart means
-- "not measured" and a bar at 0 means "measured and failing". `rated_metrics` and
-- `total_weight` are exposed because those are not the same claim — a score backed
-- by 3 of 21 metrics should not be drawn like one backed by all 21.
--
-- `components` is the per-metric {rating, weight, name} for the period, so the
-- radar can be drawn for any point on the trend, not just today.
create or replace view project_health_trend as
select f.project_id
     , f.grain
     , f.period
     , round(20.0 * sum(f.weighted) / nullif(sum(f.weight), 0))::int as health_score
     , count(*)::int   as rated_metrics
     , sum(f.weight)   as total_weight
     , jsonb_object_agg(
           f.metric
         , jsonb_build_object('rating', f.rating, 'weight', f.weight, 'name', f.metric_name)
       )               as components
       -- Share of the FULL model's weight that actually backed this score, 0-1.
       --
       -- Not cosmetic. Honest-empty (below) only distinguishes "not measured" from
       -- "measured and failing"; it says nothing about "measured, but barely". Real
       -- data has periods scored from one weight-1 metric sitting next to periods
       -- scored from nine (weight 15) — plot those together untouched and the line
       -- swings 80 → 0 → 100 on coverage artifacts, not on health. A caller should
       -- gate or fade on this rather than draw every period with equal confidence.
       --
       -- Appended last deliberately: `create or replace view` can only add columns
       -- at the end, so inserting it mid-list would force a DROP of this view and
       -- anything built on it.
     , round(
           sum(f.weight)
           / (select sum(weight) from sensei.metrics
               where rating_scale is not null and weight > 0 and direction <> 'neutral')
       , 3)            as coverage
  from sensei.metric_rating_facts f
 where f.rating is not null
 group by f.project_id, f.grain, f.period
having sum(f.weight) > 0;

comment on view project_health_trend is
'Composite 0-100 project health per (grain, period) — the bar chart + trend series.
round(20 * SUM(weight*rating)/SUM(weight)) over the period''s RATED metrics, from the
same ratings the radar uses (spec I5). Derived on read, so weight/scale tuning
re-reads history with no backfill. NO row for a period with nothing rated
(honest-empty, spec I4) — a gap means "not measured", a 0 means "measured and
failing". `coverage` (0-1) is the share of the full model''s weight behind the score:
gate or fade on it, because a period scored from one weight-1 metric will otherwise
be drawn beside one scored from nine and the line swings on coverage, not health.
`components` carries the period''s spokes so the radar works for any point on the
trend.';
