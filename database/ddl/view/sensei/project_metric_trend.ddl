set search_path to sensei, extensions;

create or replace view project_metric_trend
    as
select project_id
     , metric
     , period
     , value
     , lag(value) over (partition by project_id, metric order by period)
                                        as prior
     , value
       - lag(value) over (partition by project_id, metric order by period)
                                        as delta
     , direction
  from sensei.project_metric_weekly;

comment on view project_metric_trend is
'Weekly trend over project_metric_weekly: adds the prior period''s value and the
delta (value - prior) via lag() partitioned by project_id, metric and ordered by
period. Powers the trend arrow. prior/delta are null for a metric''s first period.
direction travels with each row so the UI can colour the delta (a positive delta on
a lower_better metric is a regression) without a registry re-join.';
