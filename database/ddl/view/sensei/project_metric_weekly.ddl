set search_path to sensei, extensions;

create or replace view project_metric_weekly
    as
select project_id
     , metric
     , date_trunc('week', date)::date as period
     , case
         when type in ('ratio', 'pct')
           then sum((props->>'numerator')::numeric)
                / nullif(sum((props->>'denominator')::numeric), 0)
         when type in ('count', 'currency')
           then sum(value)
         else (array_agg(value order by date desc))[1]
       end                            as value
     , direction
  from sensei.project_metric_daily
 group by project_id, metric, period, type, direction;

comment on view project_metric_weekly is
'Weekly roll-up of project_metric_daily, aggregating inline by metric type:
- ratio/pct: re-derive sum(numerator)/nullif(sum(denominator),0) — NEVER the mean
  of daily ratios (3 of 4 across a week reads 0.75, not the average of the dailies)
- count/currency: sum(value)
- value/score (point-in-time): the period-end value via
  (array_agg(value order by date desc))[1]
Grouped by project_id, metric, period (week start), type, direction.';
