set search_path to sensei, extensions;

-- Per-project weighted health, on 0-100 (spec R4/R10). The weighted mean of the 0-5
-- ratings (Σ w·rating / Σ w) scaled ×20 → 0-100. Only RATED metrics count (rating not
-- null → has a scale, weight > 0, non-neutral); a project with nothing rated has NO row
-- (HAVING Σw > 0) — honest-empty, never a fabricated 0. `components` carries each rated
-- metric's {rating, weight, name} so the radar + the drill-down read the same numbers
-- the score is built from. Replaces the retired project_health composite metric — health
-- is derived on read, no stored row, no Rust computer.
create or replace view project_health_score as
select project_id
     , round(20.0 * sum(weight * rating) / nullif(sum(weight), 0))::int as health_score
     , count(*)::int                                                    as rated_metrics
     , sum(weight)                                                      as total_weight
     , jsonb_object_agg(
         metric,
         jsonb_build_object('rating', rating, 'weight', weight, 'name', metric_name)
       )                                                                as components
  from sensei.metric_ratings
 where rating is not null
 group by project_id
having sum(weight) > 0;

comment on view project_health_score is
'Per-project health on 0-100 = round(20 · Σ(weight·rating) / Σweight) over the rated
metrics, with a components map (metric → {rating, weight, name}) for the radar. No row
when nothing is rated (honest-empty). Replaces the retired project_health composite.';
