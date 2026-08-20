set search_path to sensei, extensions;

-- Granular reporting base: one row per stored metric reading, denormalized with its
-- metric-registry facets (name/family/type/direction/unit/weight/rating_scale) and its
-- repository (repo_key/name). This is the flat fact table charts + aggregations query —
-- per (project, repository, metric, scope, day) at the true repo grain (NOT pooled;
-- pool via project_metric_daily or aggregate here as needed). Never fabricates: a row
-- with no repository (a project-attributed snapshot) simply has null repo columns.
create or replace view metric_facts as
select pm.project_id
     , pm.repository_id
     , r.repo_key
     , r.name          as repository_name
     , m.key           as metric
     , m.name          as metric_name
     , m.family
     , m.type
     , m.direction
     , m.unit
     , m.weight
     , m.rating_scale
     , pm.scope
     , pm.identity
     , pm.commit_sha
     , pm.computed_on
     , pm.grain
     , pm.value
     , pm.props
     , pm.source
  from sensei.project_metrics pm
  join sensei.metrics         m on m.id = pm.metric_id
  left join sensei.repositories r on r.id = pm.repository_id;

comment on view metric_facts is
'Granular fact view: project_metrics joined to its metric-registry facets and its
repository, one row per reading at repo grain. The flat base for reports/charts and
ad-hoc aggregation; the ratings/health views + project_metric_daily pool from here.';
