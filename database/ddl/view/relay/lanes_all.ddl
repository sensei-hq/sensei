set search_path to relay, extensions;

-- Live UNION ALL archive (#393). See runs_all.
create or replace view relay.lanes_all with (security_invoker = true) as
select user_id, id, crew_id, code, title, suggested_name, role, family,
       version, model, host, capabilities, repo, worktree, token_hash,
       checkpoint, registered_at, unregistered_at, limit_paused_at,
       limit_scope, limit_resumes_at, limit_note
  from relay.lanes
 union all
select user_id, id, crew_id, code, title, suggested_name, role, family,
       version, model, host, capabilities, repo, worktree, token_hash,
       checkpoint, registered_at, unregistered_at, limit_paused_at,
       limit_scope, limit_resumes_at, limit_note
  from relay_archive.lanes;

comment on view relay.lanes_all is
'Live and archived lanes. History reads use this; operational reads stay on relay.lanes.';
