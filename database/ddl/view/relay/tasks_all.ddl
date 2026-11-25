set search_path to relay, extensions;

-- Live UNION ALL archive (#393). See runs_all.
create or replace view relay.tasks_all with (security_invoker = true) as
select user_id, id, crew_id, run_id, item_id, repo_key, workspace_id, lane_id,
       review_claimed_by, review_claimed_at, title, phase, state, round,
       executor, deterministic, owned_paths, acceptance, resources, deps,
       blocked_from_state, assigned_at, state_changed_at, completed_at,
       checkpoint, steps
  from relay.tasks
 union all
select user_id, id, crew_id, run_id, item_id, repo_key, workspace_id, lane_id,
       review_claimed_by, review_claimed_at, title, phase, state, round,
       executor, deterministic, owned_paths, acceptance, resources, deps,
       blocked_from_state, assigned_at, state_changed_at, completed_at,
       checkpoint, steps
  from relay_archive.tasks;

comment on view relay.tasks_all is
'Live and archived tasks. History reads use this; operational reads stay on relay.tasks.';
