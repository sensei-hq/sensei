set search_path to sensei, extensions;

-- Compatibility view: `repository_metrics` with `project_id` restored.
--
-- The store is repository-grained (see `sensei.repository_metrics`), but a
-- project is how the app asks the question — "how is sensei doing" is a project,
-- and it may span several repositories. Rather than rewrite every dependent view
-- and read path, project_id is derived here and the whole downstream chain
-- (metric_facts, project_metric_daily → weekly/monthly/quarterly/trend,
-- metric_ratings, project_health_score …) is unchanged.
--
-- A CORRELATED SUBQUERY, not a join, and that is deliberate. A join to
-- sensei.folders fans out — a repository usually has many folder rows — and
-- would multiply every metric row by its folder count, silently inflating every
-- sum downstream. The subquery yields exactly one project per metric row.
--
-- LEFT-ish by construction: a repository with no project mapping produces
-- project_id = NULL rather than vanishing. Dropping the row would be worse — a
-- metric that exists would read as "not measured". (Measured at the rename: all
-- 67 repositories map, so this is a guard, not a live case.)
--
-- Ordering by abs_path makes the pick deterministic when a repository is
-- reachable through more than one folder, so the view cannot return different
-- project_ids for the same row across runs.
create or replace view project_metrics as
select rm.id
     , rm.metric_id
     , ( select f.project_id
           from sensei.folders f
          where f.repository_id = rm.repository_id
            and f.project_id is not null
          order by f.abs_path
          limit 1
       )                                   as project_id
     , rm.repository_id
     , rm.scope
     , rm.identity
     , rm.commit_sha
     , rm.computed_on
     , rm.grain
     , rm.value
     , rm.props
     , rm.source
     , rm.modified_at
  from sensei.repository_metrics rm;

comment on view project_metrics is
'Read-compatibility view over sensei.repository_metrics, adding the derived
project_id. WRITES GO TO THE TABLE — this view is deliberately not updatable
(the derived column has no inverse), so an attempted insert fails loudly instead
of silently targeting the wrong grain.

Dropped in the rename and NOT restored here: folder_id and session_id. Both held
zero rows across the table''s life and nothing downstream referenced them.';
