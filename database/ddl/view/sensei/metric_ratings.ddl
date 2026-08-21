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
-- The rating bands themselves live in `metric_rating_facts` (the single rating
-- expression, spec I5) — this view is now just "the newest DAILY rating per
-- (project, metric)". Keeping the bands in one place is what guarantees a radar
-- spoke and the same metric's contribution to the health score are the same
-- number; two copies of the CASE would only have to agree by inspection.
create or replace view metric_ratings as
select distinct on (f.project_id, f.metric)
       f.project_id
     , f.metric
     , f.metric_name
     , f.family
     , f.direction
     , f.weight
     , f.value
     , f.period as computed_on
     , f.rating
  from sensei.metric_rating_facts f
 where f.grain = 'daily'
 order by f.project_id, f.metric, f.period desc;

comment on view metric_ratings is
'Per (project, metric) current 0-5 rating from the latest pooled daily value against
the metric''s rating_scale (direction-aware bands; 0 worst … 5 best). NULL rating =
not rated (no scale / weight 0 / neutral). The radar reads a spoke per row here.';
