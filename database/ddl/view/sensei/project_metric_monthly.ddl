set search_path to sensei, extensions;

create or replace view project_metric_monthly
    as
select project_id
     , metric
     , date_trunc('month', date)::date as period
     , case
         when type in ('ratio', 'pct')
           then sum((props->>'numerator')::numeric)
                / nullif(sum((props->>'denominator')::numeric), 0)
         when type in ('count', 'currency')
           then sum(value)
         when type = 'duration'
           then avg(value)
         else (array_agg(value order by date desc))[1]
       end                             as value
     , direction
  from sensei.project_metric_daily
 group by project_id, metric, period, type, direction;

comment on view project_metric_monthly is
'Monthly roll-up of project_metric_daily, aggregating inline by metric type:
- ratio/pct: re-derive sum(numerator)/nullif(sum(denominator),0) — NEVER the mean
  of daily ratios
- count/currency: sum(value)
- duration: avg(value) — the mean of the daily medians (a true cross-day median is
  not recoverable from daily rows, so the mean is the pragmatic latency roll-up)
- value/score (point-in-time): the period-end value via
  (array_agg(value order by date desc))[1]
Grouped by project_id, metric, period (month start), type, direction.';
