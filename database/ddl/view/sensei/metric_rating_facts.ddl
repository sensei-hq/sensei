set search_path to sensei, extensions;

-- The ONE place a metric reading becomes a 0-5 rating, at every time grain.
--
-- Spec invariant I5 (2026-08-20-metric-rating-scales-health) requires the radar's
-- per-metric rating and the health score's per-metric rating to be the same
-- number, which only holds while there is a single rating expression. This view
-- is it: `metric_ratings` (the radar's current spokes) and the health trend both
-- read from here, so the bands cannot drift between them.
--
-- Rating = how many `rating_scale` thresholds the reading has passed, so the
-- array length sets the ceiling (5 thresholds ⇒ 0-5) and it is monotonic in the
-- improvement direction by construction (spec I3). NULL rating = NOT RATED and
-- must be excluded from a weighted mean rather than scored zero:
--   * `rating_scale` null      — nothing to rate against
--   * `weight` = 0             — deliberately not a score input (R7)
--   * `direction` = 'neutral'  — no better/worse (R7 / I2)
--   * `value` null             — no reading (I1: exclude, never rate 0)
--
-- One row per (project, metric, grain, period). Each grain rates the value ALREADY
-- rolled up for that period by the project_metric_* ladder, which is type-aware
-- (ratio/pct = Σnumerator/Σdenominator, count sums, duration averages). Rating the
-- rolled-up reading is not the same as averaging daily ratings — for any ratio
-- metric the latter would weight a quiet day equally with a busy one and drift from
-- the weekly/monthly numbers the rest of the UI shows.
--
-- `weighted` carries weight × rating so the roll-ups above are a plain SUM.
create or replace view metric_rating_facts as
with readings as (
    select d.project_id, d.metric, 'daily'::text   as grain, d.date   as period, d.value
      from sensei.project_metric_daily d
    union all
    select w.project_id, w.metric, 'weekly'::text  as grain, w.period as period, w.value
      from sensei.project_metric_weekly w
    union all
    select mo.project_id, mo.metric, 'monthly'::text as grain, mo.period as period, mo.value
      from sensei.project_metric_monthly mo
)
select r.project_id
     , r.metric
     , m.name   as metric_name
     , m.family
     , m.direction
     , m.weight
     , r.grain
     , r.period
     , r.value
     , rating.value                       as rating
     , m.weight * rating.value            as weighted
  from readings r
  join sensei.metrics m on m.key = r.metric
  cross join lateral (
      select case
               when m.rating_scale is null
                 or m.weight = 0
                 or m.direction = 'neutral'
                 or r.value is null
               then null
               when m.direction = 'higher_better' then
                 (select count(*) from jsonb_array_elements_text(m.rating_scale) as s(thr)
                   where r.value >= s.thr::float8)
               when m.direction = 'lower_better' then
                 (select count(*) from jsonb_array_elements_text(m.rating_scale) as s(thr)
                   where r.value <= s.thr::float8)
               else null
             end::int as value
  ) rating;

comment on view metric_rating_facts is
'Per (project, metric, grain, period) 0-5 rating of the period''s rolled-up reading —
the SINGLE rating expression the radar spokes and every health grain read, so they
cannot disagree (spec I5). grain is daily/weekly/monthly. NULL rating = not rated (no
scale / weight 0 / neutral direction / no reading) and must be excluded from a
weighted mean, never scored 0. `weighted` = weight * rating, so roll-ups are a SUM.';
