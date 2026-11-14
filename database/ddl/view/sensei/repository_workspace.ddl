set search_path to sensei, extensions;

-- One workspace per repository: THE single owner of the pick that
-- `workspace_metrics` documents at length (most recent membership wins, the
-- workspace uuid breaks a tie). Both readers take it from here, so the rule has
-- one home: `workspace_metrics` as a scalar subquery (it must stay auto-updatable,
-- so it cannot join), `workspace_metric_daily` as a join — which lets a
-- `workspace_id = $1` filter reach `repository_metrics` instead of resolving the
-- workspace once per metric row (measured 2026-11-02: 42k → ~1.2k buffers per
-- workspace read).
create or replace view repository_workspace as
select distinct on (pr.repository_id)
       pr.repository_id
     , pr.workspace_id
  from sensei.repositories_in_workspaces pr
 order by pr.repository_id, pr.created_at desc, pr.workspace_id;

comment on view repository_workspace is
'One workspace per repository — the most recent membership in
sensei.repositories_in_workspaces, the workspace uuid breaking a tie. ONE owner, not
THE owner: membership is multi-valued by design (see workspace_metrics). Read by
workspace_metrics (derived workspace_id) and workspace_metric_daily.';
