set search_path to sensei, extensions;

create or replace view project_metric_daily
    as
select pm.project_id
     , m.key          as metric
     , pm.computed_on as date
     , case
         when m.type in ('ratio', 'pct')
           then sum((pm.props->>'numerator')::numeric)
                / nullif(sum((pm.props->>'denominator')::numeric), 0)
         when m.type in ('count', 'currency')
           then sum(pm.value)
         when m.type = 'duration'
           then avg(pm.value)
         else (array_agg(pm.value order by pm.repository_id nulls last))[1]
       end            as value
     , case
         when m.type in ('ratio', 'pct')
           then (array_agg(pm.props order by pm.repository_id nulls last))[1]
                || jsonb_build_object(
                     'numerator'
                   , coalesce(sum((pm.props->>'numerator')::numeric), 0)
                   , 'denominator'
                   , coalesce(sum((pm.props->>'denominator')::numeric), 0))
         else (array_agg(pm.props order by pm.repository_id nulls last))[1]
       end            as props
     , m.type
     , m.direction
  from sensei.project_metrics pm
  join sensei.metrics         m
    on m.id           = pm.metric_id
 where pm.grain      = 'daily'
   and pm.scope      = 'user'
 group by pm.project_id, m.key, pm.computed_on, m.type, m.direction;

comment on view project_metric_daily is
'Base roll-up view: project x metric x date, POOLING the repo-grain value store up
to the project. project_metrics now keys on repository_id (not project_id), so a
project that includes N repositories has up to N daily rows per metric/date; this
view collapses them to one project-level row per (project, metric, date). Only the
local-user pole (scope=user) feeds the default project read; the scope=repo
whole-tree twins (churn/quality) are NOT pooled here.

Pooling is type-aware, so a multi-repo project never averages-of-averages:
- ratio/pct: sum(numerator)/nullif(sum(denominator),0) across the projects repos —
  the correct pooled ratio, NEVER the mean of per-repo ratios
- count/currency: sum(value) across repos
- duration: avg(value) across repos (the pragmatic pooled latency; a true
  cross-repo median is not recoverable from per-repo rows)
- value/score (point-in-time): the representative repo via
  (array_agg(value order by repository_id nulls last))[1]
A single-repo project is therefore an identity pass-through; a multi-repo project is
a Sum-num/Sum-den pool. Joins sensei.project_metrics to sensei.metrics to carry each
value''s type and direction, so coarser grains (weekly/monthly/quarterly/trend) can
aggregate by type without re-joining the registry, and they read this pooled view
UNCHANGED.

- metric: the stable slug from metrics.key
- date: project_metrics.computed_on (the date the value is FOR)
- props: the representative repo row (array_agg order by repository_id nulls last)[1],
  with numerator/denominator OVERWRITTEN by the pooled sums for ratio/pct so downstream
  roll-ups re-derive from sums instead of averaging; props.explainer and other keys are
  preserved from the representative row
- daily grain + scope=user only; repo-grain per-repository rows are pooled here, and
  the scope=repo whole-tree twins are excluded from the default project read.';
