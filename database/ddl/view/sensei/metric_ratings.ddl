set search_path to sensei, extensions;

-- Per (project, metric) CURRENT 0-5 rating — the radar spokes + the health inputs.
-- Source is the latest daily POOLED value per (project, metric) from
-- project_metric_daily (scope=user, Σnum/Σden across the project's repos), so a rating
-- reflects the project-level reading, not one repo. The rating = how many of the
-- metric's `rating_scale` thresholds the value has reached (direction-aware, spec R3):
--   higher_better → count(threshold <= value);  lower_better → count(threshold >= value)
-- so 0 = below the worst bar (measured-and-failing) … 5 = at/above the best bar. NULL
-- when the metric has no scale, weight 0, or a neutral direction (spec R6/R7) — those
-- carry a value but are not rated (and drop out of the health roll-up).
create or replace view metric_ratings as
with latest as (
    select distinct on (d.project_id, d.metric)
           d.project_id, d.metric, d.value, d.date
      from sensei.project_metric_daily d
     order by d.project_id, d.metric, d.date desc
)
select l.project_id
     , l.metric
     , m.name    as metric_name
     , m.family
     , m.direction
     , m.weight
     , l.value
     , l.date    as computed_on
     , case
         when m.rating_scale is null or m.weight = 0 or m.direction = 'neutral' then null
         when m.direction = 'higher_better' then
           (select count(*) from jsonb_array_elements_text(m.rating_scale) as s(thr)
             where l.value >= s.thr::float8)
         when m.direction = 'lower_better' then
           (select count(*) from jsonb_array_elements_text(m.rating_scale) as s(thr)
             where l.value <= s.thr::float8)
         else null
       end::int  as rating
  from latest l
  join sensei.metrics m on m.key = l.metric;

comment on view metric_ratings is
'Per (project, metric) current 0-5 rating from the latest pooled daily value against
the metric''s rating_scale (direction-aware bands; 0 worst … 5 best). NULL rating =
not rated (no scale / weight 0 / neutral). The radar reads a spoke per row here.';
