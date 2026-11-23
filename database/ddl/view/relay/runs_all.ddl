set search_path to relay, extensions;

-- Live UNION ALL archive, per table (#393): history reads (run list, run
-- detail, task view) see ended work identically before and after archival.
-- Operational reads stay on the live tables: archived rows are never
-- live-operated. Security-invoker like run_list: the caller's RLS applies.
create or replace view relay.runs_all with (security_invoker = true) as
select user_id, id, crew_id, title, goal, state, repo_key, workspace_id,
       config, created_at, started_at, completed_at
  from relay.runs
 union all
select user_id, id, crew_id, title, goal, state, repo_key, workspace_id,
       config, created_at, started_at, completed_at
  from relay_archive.runs;

comment on view relay.runs_all is
'Live and archived sprint runs. History reads use this; operational reads stay on relay.runs.';
