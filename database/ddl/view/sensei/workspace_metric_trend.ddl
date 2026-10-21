set search_path to sensei, extensions;

create or replace view workspace_metric_trend
    as
select workspace_id
     , metric
     , period
     , value
     , lag(value) over (partition by workspace_id, metric order by period)
                                        as prior
     , value
       - lag(value) over (partition by workspace_id, metric order by period)
                                        as delta
     , direction
  from sensei.workspace_metric_weekly;

comment on view workspace_metric_trend is
'Weekly trend over workspace_metric_weekly: adds the prior period''s value and the
delta (value - prior) via lag() partitioned by workspace_id, metric and ordered by
period. Powers the trend arrow. prior/delta are null for a metric''s first period.
direction travels with each row so the UI can colour the delta (a positive delta on
a lower_better metric is a regression) without a registry re-join.';
