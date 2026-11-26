set search_path to relay, extensions;

-- Live UNION ALL archive (#393). See runs_all.
create or replace view relay.actions_all with (security_invoker = true) as
select user_id, id, crew_id, task_id, lane_id, kind, round, phase, body_md,
       report_md, props, reply_to, created_at
  from relay.actions
 union all
select user_id, id, crew_id, task_id, lane_id, kind, round, phase, body_md,
       report_md, props, reply_to, created_at
  from relay_archive.actions;

comment on view relay.actions_all is
'Live and archived event rows. History reads use this; operational reads stay on relay.actions.';
