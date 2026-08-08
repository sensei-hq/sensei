set search_path to sensei, extensions;

create or replace view project_metric_daily
    as
select pm.project_id
     , m.key          as metric
     , pm.computed_on as date
     , pm.value
     , pm.props
     , m.type
     , m.direction
  from sensei.project_metrics pm
  join sensei.metrics         m
    on m.id           = pm.metric_id
 where pm.grain      = 'daily'
   and pm.folder_id is null;

comment on view project_metric_daily is
'Base roll-up view: project x metric x date at project scope (folder_id is null).
Joins sensei.project_metrics to sensei.metrics to carry each value''s type and
direction alongside it, so coarser grains (weekly/monthly/quarterly) can aggregate
by type without re-joining the registry.
- metric: the stable slug from metrics.key
- date: project_metrics.computed_on (the date the value is FOR)
- props: the stored parts (numerator/denominator, session_count, ...) that let
  ratio/pct roll-ups re-derive from sums instead of averaging daily ratios
- daily grain only; per-module rows (folder_id set) are excluded from this
  project-scope base view.';
